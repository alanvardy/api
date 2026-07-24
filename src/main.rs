mod app;
mod domain;
mod infra;
mod interfaces;
mod test;

use app::env::Env;
use app::state::AppState;
use axum::Router;

#[tokio::main]
async fn main() {
    app::log::init();
    let env = Env::init().await;
    let _guard = infra::sentry::init(&env.sentry_dsn);

    let pool = infra::db::init().await;
    let http_port = env.http_port;
    let address = format!("0.0.0.0:{http_port}");

    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("failed to bind server");

    tracing::info!("Server running on http://localhost:{http_port}");

    axum::serve(listener, app(pool, &env)).await.unwrap();
}

fn app(pool: sqlx::SqlitePool, env: &Env) -> Router {
    let templates = app::templates::init();
    let state = AppState {
        db: pool,
        env: env.clone(),
        templates,
    };

    interfaces::routes::routes(env)
        .layer(app::log::trace_layer())
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::fmt;

    use rust_arkitect::dsl::architectural_rules::ArchitecturalRules;
    use rust_arkitect::dsl::arkitect::Arkitect;
    use rust_arkitect::dsl::project::Project;
    use rust_arkitect::rule::Rule;
    use rust_arkitect::rust_file::RustFile;
    use syn::{
        Attribute, ExprPath, Item, ItemMod, Path, TypePath, UseTree,
        visit::{self, Visit},
    };

    #[test]
    fn test_architectural_rules() {
        Arkitect::init_logger();
        let domain_deps = vec!["base64", "chrono", "std"];

        let infra_deps = [
            vec![
                "axum",
                "sqlx",
                "sentry",
                "aws_config",
                "aws_sdk_s3",
                "api::app::error",
            ],
            domain_deps.clone(),
        ]
        .concat();

        let project = Project::from_current_crate();

        #[rustfmt::skip]
        let rules = ArchitecturalRules::define()
            .rules_for_module("api::app")
            .it_must_not_depend_on(&["api::interfaces"])
            .rules_for_module("api::interfaces")
            .it(Box::new(MustNotDependOnExceptTestsBuilder {
                forbidden: vec!["sqlx".to_string()],
            }))
            .rules_for_module("api::domain")
            .it_may_depend_on(&domain_deps)
            .rules_for_module("api::infra")
            .it_may_depend_on(&infra_deps)
            .build();

        let result = Arkitect::ensure_that(project).complies_with(rules);

        assert!(
            result.is_ok(),
            "Detected {} violations",
            result.err().unwrap().len()
        );
    }

    /// A rule that forbids dependencies except when used exclusively inside `#[cfg(test)]` modules.
    struct MustNotDependOnExceptTests {
        subject: String,
        forbidden: Vec<String>,
    }

    struct MustNotDependOnExceptTestsBuilder {
        forbidden: Vec<String>,
    }

    impl SubjectInjectableRuleBuilder for MustNotDependOnExceptTestsBuilder {
        fn for_subject(&self, subject: &str) -> Box<dyn Rule> {
            Box::new(MustNotDependOnExceptTests {
                subject: subject.to_string(),
                forbidden: self.forbidden.clone(),
            })
        }
    }

    impl fmt::Display for MustNotDependOnExceptTests {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "{} must not depend on {:?} (except in #[cfg(test)] modules)",
                self.subject, self.forbidden
            )
        }
    }

    impl Rule for MustNotDependOnExceptTests {
        fn is_applicable(&self, file: &RustFile) -> bool {
            file.logical_path.starts_with(&self.subject)
        }

        fn apply(&self, file: &RustFile) -> Result<(), String> {
            let deps = deps_outside_test_modules(&file.ast, &file.logical_path);
            let violations: Vec<_> = deps
                .iter()
                .filter(|d| {
                    self.forbidden
                        .iter()
                        .any(|f| *d == f.as_str() || d.starts_with(&format!("{}::", f)))
                })
                .collect();
            if violations.is_empty() {
                Ok(())
            } else {
                Err(format!(
                    "Forbidden dependencies in {}: {:?}",
                    file.path, violations
                ))
            }
        }
    }

    // ---------------------------------------------------------------------------
    // AST helpers
    // ---------------------------------------------------------------------------

    fn has_cfg_test(attrs: &[Attribute]) -> bool {
        attrs.iter().any(|attr| {
            attr.path().is_ident("cfg")
                && attr
                    .meta
                    .require_list()
                    .map(|list| list.tokens.to_string().trim() == "test")
                    .unwrap_or(false)
        })
    }

    /// Collect dependencies from the parts of the AST that are **not** inside
    /// `#[cfg(test)]` modules or behind `#[cfg(test)]` gates.
    fn deps_outside_test_modules(ast: &syn::File, logical_path: &str) -> Vec<String> {
        let crate_name = logical_path.split("::").next().unwrap_or("");
        let mut deps = Vec::new();
        let mut aliases: HashMap<String, String> = HashMap::new();

        for item in &ast.items {
            if has_cfg_test(&item_attrs(item)) {
                continue;
            }
            match item {
                Item::Use(use_item) => {
                    collect_use_tree(
                        &use_item.tree,
                        &mut deps,
                        &mut aliases,
                        logical_path,
                        crate_name,
                        "",
                    );
                }
                Item::Mod(mod_item) => {
                    if !has_cfg_test(&mod_item.attrs) {
                        collect_mod_items(
                            mod_item,
                            &mut deps,
                            &mut aliases,
                            logical_path,
                            crate_name,
                        );
                    }
                }
                _ => {}
            }
        }

        // Path references in code (function bodies, type annotations, etc.)
        let mut visitor = PathCollector {
            deps: Vec::new(),
            aliases: &aliases,
            logical_path,
            crate_name,
            inside_test: false,
        };
        visitor.visit_file(ast);
        deps.extend(visitor.deps);

        let mut seen = HashSet::new();
        deps.into_iter()
            .filter(|d| seen.insert(d.clone()))
            .collect()
    }

    fn item_attrs(item: &Item) -> &[Attribute] {
        match item {
            Item::Const(i) => &i.attrs,
            Item::Enum(i) => &i.attrs,
            Item::ExternCrate(i) => &i.attrs,
            Item::Fn(i) => &i.attrs,
            Item::ForeignMod(i) => &i.attrs,
            Item::Impl(i) => &i.attrs,
            Item::Macro(i) => &i.attrs,
            Item::Mod(i) => &i.attrs,
            Item::Static(i) => &i.attrs,
            Item::Struct(i) => &i.attrs,
            Item::Trait(i) => &i.attrs,
            Item::TraitAlias(i) => &i.attrs,
            Item::Type(i) => &i.attrs,
            Item::Union(i) => &i.attrs,
            Item::Use(i) => &i.attrs,
            Item::Verbatim(_) => &[],
            _ => &[],
        }
    }

    fn collect_use_tree(
        tree: &UseTree,
        deps: &mut Vec<String>,
        aliases: &mut HashMap<String, String>,
        logical_path: &str,
        crate_name: &str,
        prefix: &str,
    ) {
        match tree {
            UseTree::Path(use_path) => {
                let ident = use_path.ident.to_string();
                if ident == "super" {
                    let parent = logical_path.rsplitn(2, "::").nth(1).unwrap_or("");
                    collect_use_tree(
                        &use_path.tree,
                        deps,
                        aliases,
                        logical_path,
                        crate_name,
                        parent,
                    );
                } else if ident == "crate" {
                    collect_use_tree(
                        &use_path.tree,
                        deps,
                        aliases,
                        logical_path,
                        crate_name,
                        crate_name,
                    );
                } else {
                    let new_prefix = if prefix.is_empty() {
                        ident
                    } else {
                        format!("{}::{}", prefix, ident)
                    };
                    collect_use_tree(
                        &use_path.tree,
                        deps,
                        aliases,
                        logical_path,
                        crate_name,
                        &new_prefix,
                    );
                }
            }
            UseTree::Group(group) => {
                for item in &group.items {
                    collect_use_tree(item, deps, aliases, logical_path, crate_name, prefix);
                }
            }
            UseTree::Name(name) => {
                let dep = format!("{}::{}", prefix, name.ident);
                deps.push(dep.clone());
                aliases.insert(name.ident.to_string(), dep);
            }
            UseTree::Glob(_) => {
                deps.push(format!("{}::*", prefix));
            }
            UseTree::Rename(rename) => {
                let dep = format!("{}::{}", prefix, rename.ident);
                deps.push(dep.clone());
                aliases.insert(rename.rename.to_string(), dep);
            }
        }
    }

    fn collect_mod_items(
        mod_item: &ItemMod,
        deps: &mut Vec<String>,
        aliases: &mut HashMap<String, String>,
        logical_path: &str,
        crate_name: &str,
    ) {
        if let Some((_, items)) = &mod_item.content {
            let mod_path = format!("{}::{}", logical_path, mod_item.ident);
            for item in items {
                if has_cfg_test(&item_attrs(item)) {
                    continue;
                }
                match item {
                    Item::Use(use_item) => {
                        collect_use_tree(&use_item.tree, deps, aliases, &mod_path, crate_name, "");
                    }
                    Item::Mod(nested) => {
                        if !has_cfg_test(&nested.attrs) {
                            collect_mod_items(nested, deps, aliases, &mod_path, crate_name);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Visitor that collects path references, skipping `#[cfg(test)]` modules.
    struct PathCollector<'a> {
        deps: Vec<String>,
        aliases: &'a HashMap<String, String>,
        logical_path: &'a str,
        crate_name: &'a str,
        inside_test: bool,
    }

    impl<'ast, 'a> Visit<'ast> for PathCollector<'a> {
        fn visit_item_mod(&mut self, node: &'ast ItemMod) {
            if has_cfg_test(&node.attrs) {
                let prev = self.inside_test;
                self.inside_test = true;
                visit::visit_item_mod(self, node);
                self.inside_test = prev;
            } else {
                visit::visit_item_mod(self, node);
            }
        }

        fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
            if has_cfg_test(&node.attrs) {
                let prev = self.inside_test;
                self.inside_test = true;
                visit::visit_item_fn(self, node);
                self.inside_test = prev;
            } else {
                visit::visit_item_fn(self, node);
            }
        }

        fn visit_expr_path(&mut self, node: &'ast ExprPath) {
            if !self.inside_test {
                if let Some(dep) =
                    resolve_path(&node.path, self.aliases, self.logical_path, self.crate_name)
                {
                    self.deps.push(dep);
                }
            }
            visit::visit_expr_path(self, node);
        }

        fn visit_type_path(&mut self, node: &'ast TypePath) {
            if !self.inside_test {
                if node.path.segments.len() > 1 {
                    if let Some(dep) =
                        resolve_path(&node.path, self.aliases, self.logical_path, self.crate_name)
                    {
                        self.deps.push(dep);
                    }
                }
            }
            visit::visit_type_path(self, node);
        }
    }

    fn resolve_path(
        path: &Path,
        aliases: &HashMap<String, String>,
        logical_path: &str,
        crate_name: &str,
    ) -> Option<String> {
        let first = path.segments.first()?.ident.to_string();
        let rest: Vec<String> = path
            .segments
            .iter()
            .skip(1)
            .map(|s| s.ident.to_string())
            .collect();
        match first.as_str() {
            "crate" => Some(if rest.is_empty() {
                crate_name.to_string()
            } else {
                format!("{}::{}", crate_name, rest.join("::"))
            }),
            "super" => {
                let parent = logical_path.rsplitn(2, "::").nth(1).unwrap_or("");
                Some(if rest.is_empty() {
                    parent.to_string()
                } else {
                    format!("{}::{}", parent, rest.join("::"))
                })
            }
            "self" => Some(logical_path.to_string()),
            other => {
                if let Some(alias_target) = aliases.get(other) {
                    Some(if rest.is_empty() {
                        alias_target.clone()
                    } else {
                        format!("{}::{}", alias_target, rest.join("::"))
                    })
                } else if path.segments.len() > 1 {
                    Some(
                        path.segments
                            .iter()
                            .map(|s| s.ident.to_string())
                            .collect::<Vec<_>>()
                            .join("::"),
                    )
                } else {
                    None
                }
            }
        }
    }

    use rust_arkitect::dsl::architectural_rules::SubjectInjectableRuleBuilder;
}

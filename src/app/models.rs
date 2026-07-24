use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize)]
pub struct FeatureFlag {
    pub id: i64,
    pub name: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateFeatureFlag {
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateFeatureFlag {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Serialize, FromRow)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateUser {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentType {
    Png,
    Jpeg,
    Gif,
    Webp,
    Svg,
    Bmp,
    Tiff,
    Avif,
    Heic,
    Heif,
    Ico,
}

impl ContentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            ContentType::Png => "image/png",
            ContentType::Jpeg => "image/jpeg",
            ContentType::Gif => "image/gif",
            ContentType::Webp => "image/webp",
            ContentType::Svg => "image/svg+xml",
            ContentType::Bmp => "image/bmp",
            ContentType::Tiff => "image/tiff",
            ContentType::Avif => "image/avif",
            ContentType::Heic => "image/heic",
            ContentType::Heif => "image/heif",
            ContentType::Ico => "image/x-icon",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "image/png" => Some(ContentType::Png),
            "image/jpeg" => Some(ContentType::Jpeg),
            "image/gif" => Some(ContentType::Gif),
            "image/webp" => Some(ContentType::Webp),
            "image/svg+xml" => Some(ContentType::Svg),
            "image/bmp" => Some(ContentType::Bmp),
            "image/tiff" => Some(ContentType::Tiff),
            "image/avif" => Some(ContentType::Avif),
            "image/heic" => Some(ContentType::Heic),
            "image/heif" => Some(ContentType::Heif),
            "image/x-icon" => Some(ContentType::Ico),
            _ => None,
        }
    }
}

impl From<ContentType> for String {
    fn from(ct: ContentType) -> Self {
        ct.as_str().to_string()
    }
}

impl From<String> for ContentType {
    fn from(s: String) -> Self {
        ContentType::from_str(&s).unwrap_or_else(|| panic!("unknown content type: {}", s))
    }
}

impl TryFrom<&str> for ContentType {
    type Error = String;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        ContentType::from_str(s).ok_or_else(|| format!("unknown content type: {}", s))
    }
}

impl Serialize for ContentType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ContentType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        ContentType::from_str(&s)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown content type: {}", s)))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    pub id: i64,
    pub key: String,
    pub content_type: ContentType,
    pub user_id: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ai_flagged_at: Option<DateTime<Utc>>,
    pub human_reviewed_at: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_and_from_str_roundtrip_all_variants() {
        let variants = [
            ContentType::Png,
            ContentType::Jpeg,
            ContentType::Gif,
            ContentType::Webp,
            ContentType::Svg,
            ContentType::Bmp,
            ContentType::Tiff,
            ContentType::Avif,
            ContentType::Heic,
            ContentType::Heif,
            ContentType::Ico,
        ];
        for ct in variants {
            let s = ct.as_str();
            let parsed = ContentType::from_str(s);
            assert_eq!(parsed, Some(ct.clone()), "roundtrip failed for {s}");
        }
    }

    #[test]
    fn from_str_unknown_content_type_returns_none() {
        assert_eq!(ContentType::from_str("application/pdf"), None);
        assert_eq!(ContentType::from_str("text/plain"), None);
        assert_eq!(ContentType::from_str(""), None);
    }

    #[test]
    fn into_string_converts_to_mime() {
        let s: String = ContentType::Png.into();
        assert_eq!(s, "image/png");

        let s: String = ContentType::Svg.into();
        assert_eq!(s, "image/svg+xml");
    }

    #[test]
    fn try_from_str_happy_path() {
        let ct = ContentType::try_from("image/jpeg");
        assert_eq!(ct, Ok(ContentType::Jpeg));
    }

    #[test]
    fn try_from_str_unknown_returns_err_with_message() {
        let result = ContentType::try_from("application/octet-stream");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unknown content type"));
    }

    #[test]
    fn serialize_then_deserialize_preserves_value() {
        let original = ContentType::Webp;
        let json = serde_json::to_string(&original).expect("serialize should succeed");
        assert_eq!(json, "\"image/webp\"");

        let parsed: ContentType = serde_json::from_str(&json).expect("deserialize should succeed");
        assert_eq!(parsed, original);
    }

    #[test]
    fn deserialize_unknown_mime_fails() {
        let result: Result<ContentType, _> = serde_json::from_str("\"application/zip\"");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("unknown content type")
        );
    }

    #[test]
    fn from_string_happy_path_converts_known_mime() {
        let ct: ContentType = ContentType::from("image/gif".to_string());
        assert_eq!(ct, ContentType::Gif);
    }

    #[test]
    #[should_panic(expected = "unknown content type")]
    fn from_string_panics_on_unknown_mime() {
        let _ct: ContentType = ContentType::from("x-fake/type".to_string());
    }
}

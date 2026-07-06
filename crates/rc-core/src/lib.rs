#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseCandidate {
    pub platform: String,
    pub artist: String,
    pub title: String,
    pub album: Option<String>,
    pub duration_ms: Option<u32>,
    pub isrc: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchEvidence {
    pub field: String,
    pub score: f32,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchDecision {
    pub confidence: f32,
    pub evidence: Vec<MatchEvidence>,
}

pub fn normalize_title(input: &str) -> String {
    input
        .trim()
        .to_lowercase()
        .replace("feat.", "feat")
        .replace("ft.", "feat")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn explain_basic_match(left: &ReleaseCandidate, right: &ReleaseCandidate) -> MatchDecision {
    let left_title = normalize_title(&left.title);
    let right_title = normalize_title(&right.title);
    let title_score = if left_title == right_title { 1.0 } else { 0.0 };

    let left_artist = normalize_title(&left.artist);
    let right_artist = normalize_title(&right.artist);
    let artist_score = if left_artist == right_artist { 1.0 } else { 0.0 };

    let isrc_score = match (&left.isrc, &right.isrc) {
        (Some(a), Some(b)) if a.eq_ignore_ascii_case(b) => 1.0,
        (Some(_), Some(_)) => 0.0,
        _ => 0.5,
    };

    let confidence = (title_score * 0.45) + (artist_score * 0.35) + (isrc_score * 0.20);

    MatchDecision {
        confidence,
        evidence: vec![
            MatchEvidence {
                field: "title".to_string(),
                score: title_score,
                note: "normalized title equality".to_string(),
            },
            MatchEvidence {
                field: "artist".to_string(),
                score: artist_score,
                note: "normalized artist equality".to_string(),
            },
            MatchEvidence {
                field: "isrc".to_string(),
                score: isrc_score,
                note: "isrc exact match when both values exist".to_string(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_common_feature_marker() {
        assert_eq!(normalize_title(" Roc ft. Someone "), "roc feat someone");
    }

    #[test]
    fn explains_exact_basic_match() {
        let left = ReleaseCandidate {
            platform: "spotify".to_string(),
            artist: "2slimey".to_string(),
            title: "roc".to_string(),
            album: None,
            duration_ms: None,
            isrc: Some("US1234567890".to_string()),
            url: None,
        };
        let right = ReleaseCandidate {
            platform: "musicbrainz".to_string(),
            artist: "2Slimey".to_string(),
            title: "ROC".to_string(),
            album: None,
            duration_ms: None,
            isrc: Some("us1234567890".to_string()),
            url: None,
        };

        let decision = explain_basic_match(&left, &right);
        assert!((decision.confidence - 1.0).abs() < f32::EPSILON);
        assert_eq!(decision.evidence.len(), 3);
    }
}

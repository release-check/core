use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq)]
pub struct SourceTrack {
    pub artist: String,
    pub artist_aliases: Vec<String>,
    pub title: String,
    pub album: Option<String>,
    pub version: Option<String>,
    pub duration_ms: Option<u32>,
    pub isrc: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReleaseCandidate {
    pub platform: String,
    pub artist: String,
    pub artist_aliases: Vec<String>,
    pub title: String,
    pub album: Option<String>,
    pub version: Option<String>,
    pub duration_ms: Option<u32>,
    pub isrc: Option<String>,
    pub url: Option<String>,
    pub status: PlatformStatus,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PlatformStatus {
    Available,
    Missing,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStatus {
    Matched,
    PossibleMatch,
    Rejected,
    UnknownAvailability,
    FalsePositive,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Confidence(f32);

impl Confidence {
    pub fn new(value: f32) -> Self {
        if value.is_finite() {
            Self(value.clamp(0.0, 1.0))
        } else {
            Self(0.0)
        }
    }

    pub fn value(self) -> f32 {
        self.0
    }
}

impl Default for Confidence {
    fn default() -> Self {
        Self(0.0)
    }
}

impl From<f32> for Confidence {
    fn from(value: f32) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceField {
    Title,
    Artist,
    Album,
    Duration,
    ReleaseDate,
    Isrc,
    Url,
    Version,
    Source,
    Availability,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchEvidence {
    pub field: EvidenceField,
    pub score: Confidence,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentScores {
    pub title: Confidence,
    pub artist: Confidence,
    pub album: Confidence,
    pub version: Confidence,
    pub duration: Confidence,
    pub isrc: Confidence,
    pub source: Confidence,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchDecision {
    pub candidate_index: usize,
    pub candidate: ReleaseCandidate,
    pub status: MatchStatus,
    pub confidence: Confidence,
    pub evidence: Vec<MatchEvidence>,
    pub component_scores: ComponentScores,
    pub normalized_source: NormalizedRecord,
    pub normalized_candidate: NormalizedRecord,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedRecord {
    pub artist: String,
    pub artist_aliases: Vec<String>,
    pub title: String,
    pub album: Option<String>,
    pub version_markers: BTreeSet<VersionMarker>,
    pub isrc: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VersionMarker {
    Remix,
    Live,
    Demo,
    Remaster,
    Instrumental,
    SpedUp,
}

pub fn match_candidates(
    source: &SourceTrack,
    candidates: &[ReleaseCandidate],
) -> Vec<MatchDecision> {
    let normalized_source = normalize_source(source);
    let mut decisions = candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| score_candidate(index, source, &normalized_source, candidate))
        .collect::<Vec<_>>();

    decisions.sort_by(compare_decisions);
    decisions
}

pub fn explain_basic_match(left: &ReleaseCandidate, right: &ReleaseCandidate) -> MatchDecision {
    let source = SourceTrack {
        artist: left.artist.clone(),
        artist_aliases: left.artist_aliases.clone(),
        title: left.title.clone(),
        album: left.album.clone(),
        version: left.version.clone(),
        duration_ms: left.duration_ms,
        isrc: left.isrc.clone(),
        url: left.url.clone(),
    };

    score_candidate(0, &source, &normalize_source(&source), right)
}

pub fn normalize_title(input: &str) -> String {
    normalize_text(input)
}

pub fn normalize_artist(input: &str) -> String {
    normalize_text(input)
}

pub fn normalize_album(input: &str) -> String {
    normalize_text(input)
}

fn score_candidate(
    candidate_index: usize,
    source: &SourceTrack,
    normalized_source: &NormalizedRecord,
    candidate: &ReleaseCandidate,
) -> MatchDecision {
    let normalized_candidate = normalize_candidate(candidate);
    let title = title_score(&normalized_source.title, &normalized_candidate.title);
    let artist = artist_score(
        &normalized_source.artist,
        &normalized_source.artist_aliases,
        &normalized_candidate.artist,
        &normalized_candidate.artist_aliases,
    );
    let album = optional_text_score(&normalized_source.album, &normalized_candidate.album);
    let version = version_score(
        &normalized_source.version_markers,
        &normalized_candidate.version_markers,
    );
    let duration = duration_score(source.duration_ms, candidate.duration_ms);
    let isrc = isrc_score(&source.isrc, &candidate.isrc);
    let source_score = source_url_score(&source.url, &candidate.url);

    let component_scores = ComponentScores {
        title,
        artist,
        album,
        version,
        duration,
        isrc,
        source: source_score,
    };

    let confidence = Confidence::new(
        (title.value() * 0.28)
            + (artist.value() * 0.24)
            + (version.value() * 0.16)
            + (duration.value() * 0.10)
            + (isrc.value() * 0.16)
            + (source_score.value() * 0.06),
    );
    let status = decide_status(candidate.status, &component_scores, confidence);
    let evidence = build_evidence(candidate.status, &component_scores);

    MatchDecision {
        candidate_index,
        candidate: candidate.clone(),
        status,
        confidence,
        evidence,
        component_scores,
        normalized_source: normalized_source.clone(),
        normalized_candidate,
    }
}

fn decide_status(
    availability: PlatformStatus,
    scores: &ComponentScores,
    confidence: Confidence,
) -> MatchStatus {
    match availability {
        PlatformStatus::Unknown => MatchStatus::UnknownAvailability,
        PlatformStatus::Missing => MatchStatus::Rejected,
        PlatformStatus::Available => {
            let identity_is_strong = scores.title.value() >= 0.90 && scores.artist.value() >= 0.90;
            let version_conflict = scores.version.value() <= 0.10;
            let duration_conflict = scores.duration.value() <= 0.10;
            let isrc_conflict = scores.isrc.value() <= 0.10;

            if identity_is_strong && (version_conflict || duration_conflict || isrc_conflict) {
                MatchStatus::FalsePositive
            } else if confidence.value() >= 0.90
                && scores.title.value() >= 0.90
                && scores.artist.value() >= 0.90
                && scores.version.value() >= 0.80
            {
                MatchStatus::Matched
            } else if confidence.value() >= 0.62
                && scores.title.value() >= 0.55
                && scores.artist.value() >= 0.55
                && !version_conflict
            {
                MatchStatus::PossibleMatch
            } else {
                MatchStatus::Rejected
            }
        }
    }
}

fn build_evidence(availability: PlatformStatus, scores: &ComponentScores) -> Vec<MatchEvidence> {
    vec![
        MatchEvidence {
            field: EvidenceField::Availability,
            score: availability_score(availability),
            note: match availability {
                PlatformStatus::Available => "platform candidate is marked available",
                PlatformStatus::Missing => "platform candidate is marked missing",
                PlatformStatus::Unknown => "platform availability is unknown",
            }
            .to_string(),
        },
        MatchEvidence {
            field: EvidenceField::Title,
            score: scores.title,
            note: score_note(scores.title, "normalized title"),
        },
        MatchEvidence {
            field: EvidenceField::Artist,
            score: scores.artist,
            note: score_note(scores.artist, "normalized artist or alias"),
        },
        MatchEvidence {
            field: EvidenceField::Album,
            score: scores.album,
            note: score_note(scores.album, "normalized album"),
        },
        MatchEvidence {
            field: EvidenceField::Version,
            score: scores.version,
            note: score_note(scores.version, "version marker"),
        },
        MatchEvidence {
            field: EvidenceField::Duration,
            score: scores.duration,
            note: score_note(scores.duration, "duration"),
        },
        MatchEvidence {
            field: EvidenceField::Isrc,
            score: scores.isrc,
            note: score_note(scores.isrc, "isrc"),
        },
        MatchEvidence {
            field: EvidenceField::Source,
            score: scores.source,
            note: score_note(scores.source, "source url"),
        },
    ]
}

fn compare_decisions(left: &MatchDecision, right: &MatchDecision) -> Ordering {
    status_rank(left.status)
        .cmp(&status_rank(right.status))
        .then_with(|| {
            right
                .confidence
                .value()
                .partial_cmp(&left.confidence.value())
                .unwrap_or(Ordering::Equal)
        })
        .then_with(|| left.candidate_index.cmp(&right.candidate_index))
}

fn status_rank(status: MatchStatus) -> u8 {
    match status {
        MatchStatus::Matched => 0,
        MatchStatus::PossibleMatch => 1,
        MatchStatus::UnknownAvailability => 2,
        MatchStatus::FalsePositive => 3,
        MatchStatus::Rejected => 4,
    }
}

fn normalize_source(source: &SourceTrack) -> NormalizedRecord {
    let (title, title_markers) = normalize_title_and_markers(&source.title);
    let version_markers = merge_version_markers(title_markers, source.version.as_deref());

    NormalizedRecord {
        artist: normalize_artist(&source.artist),
        artist_aliases: normalize_aliases(&source.artist_aliases),
        title,
        album: source.album.as_deref().map(normalize_album),
        version_markers,
        isrc: normalize_isrc(&source.isrc),
    }
}

fn normalize_candidate(candidate: &ReleaseCandidate) -> NormalizedRecord {
    let (title, title_markers) = normalize_title_and_markers(&candidate.title);
    let version_markers = merge_version_markers(title_markers, candidate.version.as_deref());

    NormalizedRecord {
        artist: normalize_artist(&candidate.artist),
        artist_aliases: normalize_aliases(&candidate.artist_aliases),
        title,
        album: candidate.album.as_deref().map(normalize_album),
        version_markers,
        isrc: normalize_isrc(&candidate.isrc),
    }
}

fn normalize_aliases(aliases: &[String]) -> Vec<String> {
    let mut aliases = aliases
        .iter()
        .map(|alias| normalize_artist(alias))
        .filter(|alias| !alias.is_empty())
        .collect::<Vec<_>>();
    aliases.sort();
    aliases.dedup();
    aliases
}

fn normalize_title_and_markers(input: &str) -> (String, BTreeSet<VersionMarker>) {
    let normalized = normalize_text(input);
    let markers = version_markers(&normalized);
    let title = strip_version_words(&normalized, &markers);
    (title, markers)
}

fn merge_version_markers(
    mut markers: BTreeSet<VersionMarker>,
    explicit_version: Option<&str>,
) -> BTreeSet<VersionMarker> {
    if let Some(version) = explicit_version {
        markers.extend(version_markers(&normalize_text(version)));
    }
    markers
}

fn normalize_text(input: &str) -> String {
    let lower = input.trim().to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut last_was_space = true;

    for ch in lower.chars() {
        let mapped = match ch {
            '&' => " and ",
            '+' => " plus ",
            '\'' | '"' | '`' => "",
            _ if ch.is_alphanumeric() => {
                out.push(ch);
                last_was_space = false;
                continue;
            }
            _ => " ",
        };

        for mapped_ch in mapped.chars() {
            if mapped_ch.is_ascii_whitespace() {
                if !last_was_space {
                    out.push(' ');
                    last_was_space = true;
                }
            } else {
                out.push(mapped_ch);
                last_was_space = false;
            }
        }
    }

    out.split_whitespace()
        .map(normalize_feature_token)
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_feature_token(token: &str) -> &str {
    match token {
        "ft" | "feat" | "featuring" => "feat",
        _ => token,
    }
}

fn version_markers(text: &str) -> BTreeSet<VersionMarker> {
    let mut markers = BTreeSet::new();
    let tokens = text.split_whitespace().collect::<Vec<_>>();

    if tokens.contains(&"remix") {
        markers.insert(VersionMarker::Remix);
    }
    if tokens.contains(&"live") {
        markers.insert(VersionMarker::Live);
    }
    if tokens.contains(&"demo") {
        markers.insert(VersionMarker::Demo);
    }
    if tokens
        .iter()
        .any(|token| matches!(*token, "remaster" | "remastered"))
    {
        markers.insert(VersionMarker::Remaster);
    }
    if tokens.contains(&"instrumental") {
        markers.insert(VersionMarker::Instrumental);
    }
    if text.contains("sped up") || text.contains("speed up") {
        markers.insert(VersionMarker::SpedUp);
    }

    markers
}

fn strip_version_words(text: &str, markers: &BTreeSet<VersionMarker>) -> String {
    text.split_whitespace()
        .filter(|token| {
            !((markers.contains(&VersionMarker::Remix) && *token == "remix")
                || (markers.contains(&VersionMarker::Live) && *token == "live")
                || (markers.contains(&VersionMarker::Demo) && *token == "demo")
                || (markers.contains(&VersionMarker::Remaster)
                    && matches!(*token, "remaster" | "remastered"))
                || (markers.contains(&VersionMarker::Instrumental) && *token == "instrumental")
                || (markers.contains(&VersionMarker::SpedUp)
                    && matches!(*token, "sped" | "speed" | "up")))
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn title_score(left: &str, right: &str) -> Confidence {
    if left == right {
        Confidence::new(1.0)
    } else {
        Confidence::new(token_similarity(left, right))
    }
}

fn artist_score(
    left_artist: &str,
    left_aliases: &[String],
    right_artist: &str,
    right_aliases: &[String],
) -> Confidence {
    let mut left_names = Vec::with_capacity(left_aliases.len() + 1);
    left_names.push(left_artist);
    left_names.extend(left_aliases.iter().map(String::as_str));

    let mut right_names = Vec::with_capacity(right_aliases.len() + 1);
    right_names.push(right_artist);
    right_names.extend(right_aliases.iter().map(String::as_str));

    let best = left_names
        .iter()
        .flat_map(|left| {
            right_names
                .iter()
                .map(move |right| token_similarity(left, right))
        })
        .fold(0.0, f32::max);

    Confidence::new(best)
}

fn optional_text_score(left: &Option<String>, right: &Option<String>) -> Confidence {
    match (left, right) {
        (Some(left), Some(right)) => Confidence::new(token_similarity(left, right)),
        (None, None) => Confidence::new(0.75),
        _ => Confidence::new(0.65),
    }
}

fn version_score(left: &BTreeSet<VersionMarker>, right: &BTreeSet<VersionMarker>) -> Confidence {
    if left == right {
        Confidence::new(1.0)
    } else if !left.is_empty() && !right.is_empty() && left.intersection(right).next().is_some() {
        Confidence::new(0.40)
    } else {
        Confidence::new(0.0)
    }
}

fn duration_score(left: Option<u32>, right: Option<u32>) -> Confidence {
    match (left, right) {
        (Some(left), Some(right)) => {
            let diff = left.abs_diff(right);
            Confidence::new(match diff {
                0..=2_000 => 1.0,
                2_001..=5_000 => 0.85,
                5_001..=15_000 => 0.55,
                _ => 0.0,
            })
        }
        (None, None) => Confidence::new(0.75),
        _ => Confidence::new(0.65),
    }
}

fn isrc_score(left: &Option<String>, right: &Option<String>) -> Confidence {
    match (normalize_isrc(left), normalize_isrc(right)) {
        (Some(left), Some(right)) if left == right => Confidence::new(1.0),
        (Some(_), Some(_)) => Confidence::new(0.0),
        (None, None) => Confidence::new(0.75),
        _ => Confidence::new(0.65),
    }
}

fn source_url_score(left: &Option<String>, right: &Option<String>) -> Confidence {
    match (left, right) {
        (Some(left), Some(right)) if normalize_text(left) == normalize_text(right) => {
            Confidence::new(1.0)
        }
        (Some(_), Some(_)) => Confidence::new(0.75),
        (None, None) => Confidence::new(0.75),
        _ => Confidence::new(0.65),
    }
}

fn availability_score(status: PlatformStatus) -> Confidence {
    match status {
        PlatformStatus::Available => Confidence::new(1.0),
        PlatformStatus::Missing => Confidence::new(0.0),
        PlatformStatus::Unknown => Confidence::new(0.5),
    }
}

fn normalize_isrc(value: &Option<String>) -> Option<String> {
    value
        .as_ref()
        .map(|isrc| normalize_text(isrc).replace(' ', ""))
}

fn token_similarity(left: &str, right: &str) -> f32 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    if left == right {
        return 1.0;
    }

    let left_tokens = left.split_whitespace().collect::<BTreeSet<_>>();
    let right_tokens = right.split_whitespace().collect::<BTreeSet<_>>();
    let intersection = left_tokens.intersection(&right_tokens).count() as f32;
    let union = left_tokens.union(&right_tokens).count() as f32;

    if union == 0.0 {
        0.0
    } else {
        intersection / union
    }
}

fn score_note(score: Confidence, subject: &str) -> String {
    match score.value() {
        value if value >= 0.90 => format!("{subject} strong match"),
        value if value >= 0.60 => format!("{subject} partial or missing-data match"),
        value if value > 0.10 => format!("{subject} weak match"),
        _ => format!("{subject} conflict"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(artist: &str, title: &str) -> SourceTrack {
        SourceTrack {
            artist: artist.to_string(),
            artist_aliases: Vec::new(),
            title: title.to_string(),
            album: None,
            version: None,
            duration_ms: None,
            isrc: None,
            url: None,
        }
    }

    fn candidate(platform: &str, artist: &str, title: &str) -> ReleaseCandidate {
        ReleaseCandidate {
            platform: platform.to_string(),
            artist: artist.to_string(),
            artist_aliases: Vec::new(),
            title: title.to_string(),
            album: None,
            version: None,
            duration_ms: None,
            isrc: None,
            url: None,
            status: PlatformStatus::Available,
        }
    }

    #[test]
    fn normalizes_common_feature_marker() {
        assert_eq!(normalize_title(" Roc ft. Someone "), "roc feat someone");
        assert_eq!(
            normalize_title("ROC (featuring Someone)"),
            "roc feat someone"
        );
    }

    #[test]
    fn confidence_is_clamped_to_public_range() {
        assert_eq!(Confidence::new(-0.25).value(), 0.0);
        assert_eq!(Confidence::new(1.25).value(), 1.0);
        assert_eq!(Confidence::new(f32::NAN).value(), 0.0);
    }

    #[test]
    fn exact_match_is_matched_with_required_evidence() {
        let mut source = source("2slimey", "roc");
        source.duration_ms = Some(180_000);
        source.isrc = Some("US1234567890".to_string());
        source.url = Some("https://example.test/source/roc".to_string());

        let mut candidate = candidate("spotify", "2Slimey", "ROC");
        candidate.duration_ms = Some(181_000);
        candidate.isrc = Some("us1234567890".to_string());
        candidate.url = Some("https://open.spotify.test/track/roc".to_string());

        let decisions = match_candidates(&source, &[candidate]);
        let decision = &decisions[0];

        assert_eq!(decision.status, MatchStatus::Matched);
        assert!(decision.confidence.value() >= 0.90);
        assert!(decision
            .evidence
            .iter()
            .any(|evidence| evidence.field == EvidenceField::Title));
        assert!(decision
            .evidence
            .iter()
            .any(|evidence| evidence.field == EvidenceField::Source));
    }

    #[test]
    fn alias_match_can_be_matched() {
        let mut source = source("Diddy", "Bad Boy for Life");
        source.artist_aliases = vec!["Puff Daddy".to_string(), "Sean Combs".to_string()];

        let candidate = candidate("musicbrainz", "Sean Combs", "bad boy for life");
        let decision = &match_candidates(&source, &[candidate])[0];

        assert_eq!(decision.status, MatchStatus::Matched);
        assert_eq!(decision.component_scores.artist.value(), 1.0);
    }

    #[test]
    fn casing_and_spacing_do_not_change_title_identity() {
        let source = source("Museum Music", "  Night   Drive ");
        let candidate = candidate("discogs", "museum music", "night drive");

        let decision = &match_candidates(&source, &[candidate])[0];

        assert_eq!(decision.status, MatchStatus::Matched);
        assert_eq!(decision.component_scores.title.value(), 1.0);
    }

    #[test]
    fn unicode_titles_and_artists_are_not_collapsed_to_empty_identity() {
        let source = source("아이유", "밤편지");
        let candidate = candidate("melon", "뉴진스", "디토");

        let decision = &match_candidates(&source, &[candidate])[0];

        assert_eq!(normalize_artist("아이유"), "아이유");
        assert_eq!(normalize_title("밤편지"), "밤편지");
        assert_eq!(decision.component_scores.artist.value(), 0.0);
        assert_eq!(decision.component_scores.title.value(), 0.0);
        assert_eq!(decision.status, MatchStatus::Rejected);
    }

    #[test]
    fn version_mismatch_is_available_false_positive() {
        let source = source("2slimey", "roc");
        let candidate = candidate("spotify", "2slimey", "roc remix");

        let decision = &match_candidates(&source, &[candidate])[0];

        assert_eq!(decision.status, MatchStatus::FalsePositive);
        assert_eq!(decision.component_scores.version.value(), 0.0);
        assert!(decision.confidence.value() < 0.90);
    }

    #[test]
    fn duration_mismatch_is_evidence_against_identity() {
        let mut source = source("2slimey", "roc");
        source.duration_ms = Some(180_000);
        let mut candidate = candidate("spotify", "2slimey", "roc");
        candidate.duration_ms = Some(260_000);

        let decision = &match_candidates(&source, &[candidate])[0];

        assert_eq!(decision.status, MatchStatus::FalsePositive);
        assert_eq!(decision.component_scores.duration.value(), 0.0);
    }

    #[test]
    fn unknown_platform_availability_is_not_promoted_to_match() {
        let source = source("2slimey", "roc");
        let mut candidate = candidate("youtube", "2slimey", "roc");
        candidate.status = PlatformStatus::Unknown;

        let decision = &match_candidates(&source, &[candidate])[0];

        assert_eq!(decision.status, MatchStatus::UnknownAvailability);
        assert!(decision.confidence.value() >= 0.90);
    }

    #[test]
    fn deterministic_fixture_locks_order_status_ranges_and_components() {
        let mut source = source("Museum Music", "Night Drive");
        source.artist_aliases = vec!["MM".to_string()];
        source.duration_ms = Some(200_000);
        source.isrc = Some("USRC17607839".to_string());

        let mut exact = candidate("spotify", "museum music", "night drive");
        exact.duration_ms = Some(201_000);
        exact.isrc = Some("USRC17607839".to_string());

        let remix = candidate("soundcloud", "Museum Music", "Night Drive Remix");

        let mut unknown = candidate("youtube", "MM", "night drive");
        unknown.status = PlatformStatus::Unknown;

        let rejected = candidate("discogs", "Other Artist", "Night Ride");

        let decisions = match_candidates(&source, &[remix, unknown, rejected, exact]);

        assert_eq!(decisions[0].candidate.platform, "spotify");
        assert_eq!(decisions[0].status, MatchStatus::Matched);
        assert!(decisions[0].confidence.value() >= 0.90);
        assert_eq!(decisions[0].component_scores.isrc.value(), 1.0);

        assert_eq!(decisions[1].candidate.platform, "youtube");
        assert_eq!(decisions[1].status, MatchStatus::UnknownAvailability);
        assert!(decisions[1].confidence.value() >= 0.80);

        assert_eq!(decisions[2].candidate.platform, "soundcloud");
        assert_eq!(decisions[2].status, MatchStatus::FalsePositive);
        assert_eq!(decisions[2].component_scores.version.value(), 0.0);

        assert_eq!(decisions[3].candidate.platform, "discogs");
        assert_eq!(decisions[3].status, MatchStatus::Rejected);
    }

    #[test]
    fn explain_basic_match_keeps_single_candidate_compatibility() {
        let mut left = candidate("spotify", "2slimey", "roc");
        left.isrc = Some("US1234567890".to_string());

        let mut right = candidate("musicbrainz", "2Slimey", "ROC");
        right.isrc = Some("us1234567890".to_string());

        let decision = explain_basic_match(&left, &right);

        assert_eq!(decision.status, MatchStatus::Matched);
        assert_eq!(decision.evidence.len(), 8);
    }
}

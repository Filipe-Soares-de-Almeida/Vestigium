use std::collections::{BTreeSet, HashMap};

use vestigium_core::health::FindingKind;
use vestigium_core::hwtest::{BenchMode, Bus, Outcome, PATTERNS, TestKind, VolumeResult};
use vestigium_core::{
    Applier, Cause, ColorTheme, Confidence, DetailLevel, DumpKind, EpisodeKind, EventMeaning,
    GapReason, LanguagePref, Media, Metric, OccurrenceKind, PcState, RunAs, Severity, Signature,
    TWEAKS, Takes, ThemePref,
};
use vestigium_i18n::{Catalog, Lang};

fn keys(lang: Lang) -> BTreeSet<String> {
    Catalog::load(lang).keys().map(str::to_string).collect()
}

/// The variant's name as the window writes it into a key: the text before any fields.
fn name<T: std::fmt::Debug>(variant: &T) -> String {
    format!("{variant:?}")
        .split(|c: char| !c.is_alphanumeric())
        .next()
        .unwrap_or_default()
        .to_string()
}

fn names<T: std::fmt::Debug>(variants: &[T]) -> Vec<String> {
    variants.iter().map(name).collect()
}

/// Every value of an enum with fields, one per variant, walked through a match that stops
/// compiling when a variant is added.
fn walk<T>(first: T, next: impl Fn(&T) -> Option<T>) -> Vec<T> {
    std::iter::successors(Some(first), |value| next(value)).collect()
}

fn every_cause() -> Vec<Cause> {
    walk(Cause::Storage { disk: None }, |cause| match cause {
        Cause::Storage { .. } => Some(Cause::DiskIo {
            disk: String::new(),
            hdd: false,
            process: None,
        }),
        Cause::DiskIo { .. } => Some(Cause::Memory { process: None }),
        Cause::Memory { .. } => Some(Cause::Cpu {
            processes: Vec::new(),
        }),
        Cause::Cpu { .. } => Some(Cause::Driver { core: 0 }),
        Cause::Driver { .. } => Some(Cause::GpuDriver),
        Cause::GpuDriver => Some(Cause::Throttling),
        Cause::Throttling => Some(Cause::App {
            name: String::new(),
        }),
        Cause::App { .. } => Some(Cause::Unclear),
        Cause::Unclear => Some(Cause::DriverModule {
            module: String::new(),
            intermediary: false,
        }),
        Cause::DriverModule { .. } => Some(Cause::Frames { app: String::new() }),
        Cause::Frames { .. } => Some(Cause::BlueScreen { code: 0 }),
        Cause::BlueScreen { .. } => Some(Cause::Unresponsive),
        Cause::Unresponsive => Some(Cause::SleepTransition),
        Cause::SleepTransition => Some(Cause::PowerCut { no_dump: false }),
        Cause::PowerCut { .. } => Some(Cause::HardwareFault),
        Cause::HardwareFault => Some(Cause::NotMeasured),
        Cause::NotMeasured => None,
    })
}

fn every_run_as() -> Vec<RunAs> {
    walk(RunAs::System, |run_as| match run_as {
        RunAs::System => Some(RunAs::LocalService),
        RunAs::LocalService => Some(RunAs::NetworkService),
        RunAs::NetworkService => Some(RunAs::CurrentUser { elevated: false }),
        RunAs::CurrentUser { .. } => Some(RunAs::OtherUser),
        RunAs::OtherUser => Some(RunAs::Unknown),
        RunAs::Unknown => None,
    })
}

fn every_signature() -> Vec<Signature> {
    let first = Signature::Valid {
        signer: String::new(),
    };
    walk(first, |signature| match signature {
        Signature::Valid { .. } => Some(Signature::Invalid),
        Signature::Invalid => Some(Signature::Unsigned),
        Signature::Unsigned => Some(Signature::Unknown),
        Signature::Unknown => None,
    })
}

/// The episodes the window words; the two hints are drawn without a catalog key.
fn episodes_with_a_key() -> Vec<EpisodeKind> {
    let every = walk(EpisodeKind::MemoryHigh, |kind| match kind {
        EpisodeKind::MemoryHigh => Some(EpisodeKind::DiskBusy {
            disk: String::new(),
        }),
        EpisodeKind::DiskBusy { .. } => Some(EpisodeKind::CpuSaturated),
        EpisodeKind::CpuSaturated => Some(EpisodeKind::DpcIsrHint { core: 0 }),
        EpisodeKind::DpcIsrHint { .. } => Some(EpisodeKind::DiskLatencyHint {
            disk: String::new(),
        }),
        EpisodeKind::DiskLatencyHint { .. } => None,
    });
    every.into_iter().filter(|kind| !kind.is_hint()).collect()
}

/// Keys the window builds as `prefix.<variant>`, with `.suffix` after the variant when set.
struct Family {
    prefix: &'static str,
    suffix: &'static str,
    variants: Vec<String>,
}

fn family(prefix: &'static str, variants: Vec<String>) -> Family {
    Family {
        prefix,
        suffix: "",
        variants,
    }
}

fn suffixed(prefix: &'static str, suffix: &'static str, variants: Vec<String>) -> Family {
    Family {
        prefix,
        suffix,
        variants,
    }
}

fn lowercase(variants: Vec<String>) -> Vec<String> {
    variants
        .iter()
        .map(|variant| variant.to_lowercase())
        .collect()
}

/// Every key the window builds from an enum's variant name, with the variants of that enum.
fn families() -> Vec<Family> {
    let kinds = names(&OccurrenceKind::ALL);
    let gaps = names(&GapReason::ALL);
    let tests = names(&TestKind::ALL);
    let checks = names(&VolumeResult::ALL);
    let media = names(&Media::ALL);
    let modes = names(&BenchMode::ALL);
    let details = names(&DetailLevel::ALL);
    let dumps = names(&DumpKind::ALL);
    let tweaks = names(&TWEAKS);
    let findings = names(&FindingKind::ALL);
    vec![
        family("ui.occurrence.event", names(&EventMeaning::ALL)),
        family("ui.common.kind", kinds.clone()),
        family("ui.now.last", kinds.clone()),
        family("ui.today.title", kinds),
        family("ui.common.severity", names(&Severity::ALL)),
        family("ui.today.states.gap", gaps.clone()),
        family("ui.occurrence.fact.gap", gaps),
        family("ui.occurrence.test", tests.clone()),
        family("ui.detail.test", tests),
        family("ui.detail.outcome", names(&Outcome::ALL)),
        family("ui.occurrence.confidence", names(&Confidence::ALL)),
        family("ui.occurrence.metric", names(&Metric::ALL)),
        family("ui.common.state", lowercase(names(&PcState::ALL))),
        family("ui.settings.language", names(&LanguagePref::ALL)),
        family("ui.settings.appearance.mode", names(&ThemePref::ALL)),
        family("ui.common.color_theme", names(&ColorTheme::ALL)),
        family("ui.common.detail", details.clone()),
        suffixed("ui.common.detail", "hint", details),
        family("ui.tests.disk.name", media.clone()),
        family("ui.tests.bench.range", media),
        family("ui.tests.bus", names(&Bus::ALL)),
        family("ui.tests.check", checks.clone()),
        family("ui.detail.check", checks),
        family("ui.tests.pattern", names(&PATTERNS)),
        family("ui.tests.bench.mode", modes.clone()),
        suffixed("ui.tests.bench.mode", "tech", modes),
        family("ui.dumps.kind", dumps.clone()),
        suffixed("ui.dumps.kind", "tech", dumps),
        family("ui.tweaks.applier", names(&Applier::ALL)),
        family("ui.tweaks.takes", names(&Takes::ALL)),
        family("ui.tweaks.name", tweaks.clone()),
        suffixed("ui.tweaks.name", "tech", tweaks.clone()),
        family("ui.tweaks.what", tweaks.clone()),
        suffixed("ui.tweaks.what", "tech", tweaks.clone()),
        family("ui.tweaks.risk", tweaks.clone()),
        family("ui.tweaks.why_not", tweaks),
        family("ui.occurrence.summary", names(&every_cause())),
        family("ui.detail.process.run_as", names(&every_run_as())),
        family("ui.detail.process.signature", names(&every_signature())),
        family("ui.now.tech.episode", names(&episodes_with_a_key())),
        suffixed("ui.health.kind", "tech", findings.clone()),
        suffixed("ui.health.kind", "rule", findings),
    ]
}

fn key_of(family: &Family, variant: &str) -> String {
    match family.suffix {
        "" => format!("{}.{variant}", family.prefix),
        suffix => format!("{}.{variant}.{suffix}", family.prefix),
    }
}

#[test]
fn every_displayed_variant_has_text_in_both_languages() {
    for lang in [Lang::En, Lang::PtBr] {
        let keys = keys(lang);
        let missing: Vec<String> = families()
            .iter()
            .flat_map(|family| {
                family
                    .variants
                    .iter()
                    .map(|variant| key_of(family, variant))
                    .collect::<Vec<_>>()
            })
            .filter(|key| !keys.contains(key))
            .collect();
        assert!(missing.is_empty(), "{lang:?}: missing {missing:?}");
    }
}

#[test]
fn every_key_family_lists_variants() {
    for family in families() {
        assert!(
            !family.variants.is_empty(),
            "{} lists no variant",
            family.prefix
        );
    }
}

#[test]
fn a_walk_over_an_enum_with_fields_reaches_every_variant() {
    assert_eq!(every_cause().len(), 17);
    assert_eq!(every_run_as().len(), 6);
    assert_eq!(every_signature().len(), 4);
    assert_eq!(episodes_with_a_key().len(), 3);
}

#[test]
fn catalogs_have_the_same_keys() {
    let en = keys(Lang::En);
    let pt = keys(Lang::PtBr);
    assert!(!en.is_empty());
    assert_eq!(
        en.difference(&pt).collect::<Vec<_>>(),
        Vec::<&String>::new(),
        "missing in pt-BR"
    );
    assert_eq!(
        pt.difference(&en).collect::<Vec<_>>(),
        Vec::<&String>::new(),
        "missing in en"
    );
}

#[test]
fn no_value_is_empty() {
    for lang in [Lang::En, Lang::PtBr] {
        let catalog = Catalog::load(lang);
        for key in catalog.keys() {
            assert!(
                !catalog.get(key).trim().is_empty(),
                "{lang:?}: {key} is empty"
            );
        }
    }
}

#[test]
fn each_language_has_its_own_text() {
    let en = Catalog::load(Lang::En);
    let pt = Catalog::load(Lang::PtBr);
    assert_ne!(en.get("tray.exit"), pt.get("tray.exit"));
}

#[test]
fn missing_key_falls_back_to_en() {
    let en = HashMap::from([("a".to_string(), "alpha".to_string())]);
    let other = HashMap::from([("b".to_string(), "beta".to_string())]);
    let catalog = Catalog::from_maps(other, en);

    assert_eq!(catalog.get("a"), "alpha");
    assert_eq!(catalog.get("b"), "beta");
}

#[test]
fn unknown_key_returns_the_key() {
    assert_eq!(Catalog::load(Lang::En).get("no.such.key"), "no.such.key");
}

#[test]
fn from_langid_maps_portuguese() {
    assert_eq!(Lang::from_langid(0x0416), Lang::PtBr);
    assert_eq!(Lang::from_langid(0x0816), Lang::PtBr);
    assert_eq!(Lang::from_langid(0x0409), Lang::En);
    assert_eq!(Lang::from_langid(0x0C0A), Lang::En);
}

#[test]
fn format_replaces_placeholders() {
    let en = HashMap::from([("k".to_string(), "x {error} y {error}".to_string())]);
    let catalog = Catalog::from_maps(HashMap::new(), en);

    assert_eq!(catalog.format("k", &[("error", "boom")]), "x boom y boom");
}

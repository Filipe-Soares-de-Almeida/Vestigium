use std::time::Duration;

use vestigium_core::{CURATED, Channel, CuratedEvent, EventMeaning, classify, query_xml_for};

#[test]
fn every_curated_entry_names_its_providers() {
    for entry in CURATED {
        assert!(
            !entry.providers.is_empty(),
            "{:?} has no provider",
            entry.meaning
        );
        assert!(!entry.ids.is_empty(), "{:?} has no event id", entry.meaning);
        for provider in entry.providers {
            assert!(!provider.trim().is_empty());
            assert!(
                !provider.contains(['\'', '"']),
                "{provider} would break the XPath quoting"
            );
        }
    }
}

#[test]
fn classify_needs_provider_and_id() {
    assert_eq!(
        classify(Channel::System, "storahci", 129),
        Some(EventMeaning::StorageReset)
    );
    assert_eq!(
        classify(Channel::System, "iaStorAVC", 129),
        Some(EventMeaning::StorageReset)
    );
    assert_eq!(
        classify(Channel::System, "STORNVME", 129),
        Some(EventMeaning::StorageReset),
        "provider names compare without case"
    );
    assert_eq!(
        classify(Channel::System, "Microsoft-Windows-Hyper-V-Hypervisor", 129),
        None
    );
    assert_eq!(
        classify(Channel::Application, "Application Error", 1000),
        Some(EventMeaning::AppCrash)
    );
    assert_eq!(
        classify(Channel::Application, "Dell DcfRedirection", 1000),
        None
    );
    assert_eq!(classify(Channel::System, "Application Error", 1000), None);
    assert_eq!(classify(Channel::System, "disk", 129), None);
    assert_eq!(
        classify(Channel::System, "disk", 153),
        Some(EventMeaning::DiskRetry)
    );
    assert_eq!(
        classify(Channel::System, "volmgr", 46),
        Some(EventMeaning::NoCrashDump)
    );
    assert_eq!(classify(Channel::System, "disk", 46), None);
}

#[test]
fn query_xml_is_one_select_per_entry() {
    let xml = query_xml_for(CURATED, None);

    assert!(xml.starts_with("<QueryList><Query Id=\"0\">"));
    assert!(xml.ends_with("</Query></QueryList>"));
    assert_eq!(xml.matches("<Select Path=").count(), CURATED.len());
    assert_eq!(xml.matches("</Select>").count(), CURATED.len());
    assert!(xml.contains(
        "<Select Path=\"System\">*[System[(Provider[@Name='Display']) and (EventID=4101)]]</Select>"
    ));
    assert!(xml.contains("@Name='Service Control Manager'"));
    assert!(xml.contains("(EventID=7031 or EventID=7034)"));
    assert!(!xml.contains("timediff"));
}

#[test]
fn query_xml_adds_timediff_for_backfill() {
    let xml = query_xml_for(CURATED, Some(Duration::from_secs(7 * 24 * 3600)));

    assert_eq!(
        xml.matches("TimeCreated[timediff(@SystemTime) &lt;= 604800000]")
            .count(),
        CURATED.len()
    );
}

#[test]
fn query_xml_escapes_xml_characters() {
    const ODD: &[CuratedEvent] = &[CuratedEvent {
        channel: Channel::Application,
        providers: &["R&D <Tools>"],
        ids: &[1],
        meaning: EventMeaning::AppCrash,
    }];

    let xml = query_xml_for(ODD, None);

    assert!(xml.contains("@Name='R&amp;D &lt;Tools&gt;'"));
    assert!(xml.contains("<Select Path=\"Application\">"));
}

#[test]
fn memory_diagnostic_event_is_curated() {
    for id in [1101, 1102, 1103, 1104, 1201, 1202] {
        assert_eq!(
            classify(
                Channel::System,
                "Microsoft-Windows-MemoryDiagnostics-Results",
                id
            ),
            Some(EventMeaning::MemoryDiagnostic)
        );
    }
    assert_eq!(
        classify(
            Channel::System,
            "Microsoft-Windows-MemoryDiagnostics-Results",
            2001
        ),
        None
    );
}

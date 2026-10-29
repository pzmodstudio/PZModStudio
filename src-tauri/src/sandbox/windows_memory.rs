//! When Windows itself ran out of memory with the game open: the Resource-Exhaustion-Detector
//! event (System log, id 2004) says the commit limit, what was charged against it and which
//! processes held the most, and the Application Error event (id 1000) says which program
//! Windows then closed. A game that "closed on its own" with no Lua error usually died here.
//!
//! Read through `wevtutil` (part of Windows), as XML, parsed by string search: the two events
//! have a fixed shape and the fields wanted are few.

use serde::{Deserialize, Serialize};

const GAME_EXE: &str = "ProjectZomboid64.exe";
/// A crash this close to the memory event is taken as its consequence.
const CRASH_WINDOW_SECS: i64 = 10 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProcessCharge {
    pub name: String,
    pub gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LowMemoryEvent {
    /// Unix seconds.
    pub at: i64,
    /// What the system may commit in all (RAM plus page file), in GB, and how much was.
    pub commit_limit_gb: f64,
    pub commit_charge_gb: f64,
    pub physical_gb: f64,
    /// The game's own commit charge, when it was among the processes named.
    pub game_gb: Option<f64>,
    /// The other processes named, biggest first.
    pub others: Vec<ProcessCharge>,
    /// Windows closed the game within minutes of this event.
    pub game_crashed: bool,
}

fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let i = s.find(open)? + open.len();
    let j = s[i..].find(close)? + i;
    Some(&s[i..j])
}

fn attr_time(event: &str) -> Option<i64> {
    let iso = between(event, "<TimeCreated SystemTime='", "'")?;
    chrono::DateTime::parse_from_rfc3339(iso).ok().map(|t| t.timestamp())
}

fn gb(bytes: &str) -> f64 {
    bytes.trim().parse::<f64>().unwrap_or(0.0) / 1024.0 / 1024.0 / 1024.0
}

/// The events in one `wevtutil ... /f:xml` output.
fn split_events(xml: &str) -> Vec<&str> {
    xml.split("<Event ").filter(|e| e.contains("<EventID>")).collect()
}

/// One Resource-Exhaustion-Detector event's numbers.
pub fn parse_low_memory(event: &str) -> Option<LowMemoryEvent> {
    let at = attr_time(event)?;
    let info = between(event, "<SystemInfo>", "</SystemInfo>")?;
    let commit_limit_gb = gb(between(info, "<SystemCommitLimit>", "<")?);
    let commit_charge_gb = gb(between(info, "<SystemCommitCharge>", "<")?);
    let physical_gb = gb(between(info, "<PhysicalMemorySize>", "<").unwrap_or("0"));
    let mut game_gb = None;
    // A program with several processes (a browser) is one line, its processes summed.
    let mut by_name: Vec<ProcessCharge> = Vec::new();
    if let Some(procs) = between(event, "<ProcessInfo>", "</ProcessInfo>") {
        for block in procs.split("<Process_").skip(1) {
            let Some(name) = between(block, "<Name>", "</Name>").map(str::trim).filter(|n| !n.is_empty()) else { continue };
            let charge = between(block, "<CommitCharge>", "</CommitCharge>").map(gb).unwrap_or(0.0);
            if name.eq_ignore_ascii_case(GAME_EXE) {
                game_gb = Some(game_gb.unwrap_or(0.0) + charge);
            } else if let Some(p) = by_name.iter_mut().find(|p| p.name.eq_ignore_ascii_case(name)) {
                p.gb += charge;
            } else {
                by_name.push(ProcessCharge { name: name.to_string(), gb: charge });
            }
        }
    }
    let mut others = by_name;
    others.sort_by(|a, b| b.gb.partial_cmp(&a.gb).unwrap_or(std::cmp::Ordering::Equal));
    others.truncate(3);
    Some(LowMemoryEvent { at, commit_limit_gb, commit_charge_gb, physical_gb, game_gb, others, game_crashed: false })
}

/// When Windows closed the game (Application Error events naming it).
pub fn parse_game_crashes(xml: &str) -> Vec<i64> {
    split_events(xml)
        .into_iter()
        .filter(|e| between(e, "<Data Name='AppName'>", "</Data>").is_some_and(|n| n.eq_ignore_ascii_case(GAME_EXE)))
        .filter_map(attr_time)
        .collect()
}

/// The low-memory events in `system_xml`, with the game's crashes in `app_xml` matched to them,
/// newest first.
pub fn events_from(system_xml: &str, app_xml: &str, since: i64) -> Vec<LowMemoryEvent> {
    let crashes = parse_game_crashes(app_xml);
    let mut out: Vec<LowMemoryEvent> = split_events(system_xml)
        .into_iter()
        .filter_map(parse_low_memory)
        .filter(|e| e.at >= since)
        .map(|mut e| {
            e.game_crashed = crashes.iter().any(|c| (c - e.at).abs() <= CRASH_WINDOW_SECS);
            e
        })
        .collect();
    out.sort_by(|a, b| b.at.cmp(&a.at));
    out
}

/// The Monitor card for one event: what held the memory and what to do, in the player's words
/// through `monitor.cards.windows_low_memory`.
pub fn low_memory_card(e: &LowMemoryEvent) -> super::TranslatedErrorPayload {
    let when = chrono::DateTime::<chrono::Local>::from(std::time::UNIX_EPOCH + std::time::Duration::from_secs(e.at.max(0) as u64)).format("%Y-%m-%d %H:%M").to_string();
    let others = e.others.iter().map(|o| format!("{} {:.1} GB", o.name.trim_end_matches(".exe"), o.gb)).collect::<Vec<_>>().join(", ");
    let game = e.game_gb.map(|g| format!("{:.1}", g)).unwrap_or_else(|| "?".to_string());
    let charge = format!("{:.0}", e.commit_charge_gb);
    let limit = format!("{:.0}", e.commit_limit_gb);
    let ram = format!("{:.0}", e.physical_gb);
    super::TranslatedErrorPayload {
        id: format!("lowmem_{}", e.at),
        raw_error: format!("Windows Resource-Exhaustion-Detector 2004 at {}: {} of {} GB committed, {} {} GB", when, charge, limit, GAME_EXE, game),
        source_file: None,
        line_number: None,
        title: "Windows closed the game for lack of memory".to_string(),
        explanation: format!(
            "At {} the whole PC had {} of {} GB committed (RAM {} GB plus the page file); the game alone held {} GB{}. Windows then closed it. This is not the game's Java heap (-Xmx): the game's native side and Java mods grow past it.",
            when, charge, limit, ram, game, if others.is_empty() { String::new() } else { format!(", beside {}", others) }
        ),
        recommended_action: "A bigger page file (Windows > System > About > Advanced system settings > Performance > Virtual memory: system managed, or 16 to 32 GB), closing browsers and other big programs before playing, and fewer memory-hungry Java mods. Health > Performance > Memory keeps the last 14 days.".to_string(),
        polyfill_rule_id_suggestion: None,
        mod_name: None,
        severity: super::Severity::Exception,
        code: "windows_low_memory".to_string(),
        params: crate::catalog::issue_params(&[("when", &when), ("charge", &charge), ("limit", &limit), ("ram", &ram), ("game", &game), ("others", &others), ("othersKey", if others.is_empty() { "" } else { "beside_others" })]),
        called_from: Vec::new(),
    }
}

/// The event behind a game that just closed, when Windows logged one in the last ten minutes.
pub fn event_behind_exit() -> Option<LowMemoryEvent> {
    let now = chrono::Utc::now().timestamp();
    recent(1).into_iter().find(|e| now - e.at <= 10 * 60)
}

#[cfg(windows)]
fn wevtutil(log: &str, query: &str, count: u32) -> String {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    std::process::Command::new("wevtutil")
        .args(["qe", log, &format!("/q:{}", query), "/f:xml", &format!("/c:{}", count), "/rd:true"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

#[cfg(not(windows))]
fn wevtutil(_log: &str, _query: &str, _count: u32) -> String {
    String::new()
}

/// The low-memory events of the last `days`, the game's crashes matched to them. Empty off
/// Windows, or when there were none.
pub fn recent(days: i64) -> Vec<LowMemoryEvent> {
    let since = chrono::Utc::now().timestamp() - days * 86_400;
    let system = wevtutil("System", "*[System[Provider[@Name='Microsoft-Windows-Resource-Exhaustion-Detector'] and (EventID=2004)]]", 20);
    if !system.contains("<EventID>") {
        return Vec::new();
    }
    let app = wevtutil("Application", "*[System[Provider[@Name='Application Error'] and (EventID=1000)]]", 80);
    events_from(&system, &app, since)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SYSTEM: &str = "<Event xmlns='x'><System><Provider Name='Microsoft-Windows-Resource-Exhaustion-Detector'/><EventID>2004</EventID><TimeCreated SystemTime='2026-10-28T16:42:17.3751964Z'/></System><UserData><MemoryExhaustionInfo><SystemInfo><SystemCommitLimit>66046926848</SystemCommitLimit><SystemCommitCharge>66005184512</SystemCommitCharge><ProcessCommitCharge>43128356864</ProcessCommitCharge><PhysicalMemorySize>34180575232</PhysicalMemorySize></SystemInfo><ProcessInfo><Process_1><Name>ProjectZomboid64.exe</Name><ID>24832</ID><CommitCharge>26432516096</CommitCharge></Process_1><Process_2><Name>opera.exe</Name><ID>1</ID><CommitCharge>3221225472</CommitCharge></Process_2><Process_3><Name>Code.exe</Name><ID>2</ID><CommitCharge>1073741824</CommitCharge></Process_3><Process_4><Name>opera.exe</Name><ID>3</ID><CommitCharge>1073741824</CommitCharge></Process_4><Process_5><Name></Name><ID>4</ID><CommitCharge>5</CommitCharge></Process_5></ProcessInfo></MemoryExhaustionInfo></UserData></Event><Event xmlns='x'><System><Provider Name='Microsoft-Windows-Resource-Exhaustion-Detector'/><EventID>2004</EventID><TimeCreated SystemTime='2026-09-01T10:00:00.0000000Z'/></System><UserData><MemoryExhaustionInfo><SystemInfo><SystemCommitLimit>1</SystemCommitLimit><SystemCommitCharge>1</SystemCommitCharge></SystemInfo></MemoryExhaustionInfo></UserData></Event>";
    const APP: &str = "<Event xmlns='x'><System><Provider Name='Application Error'/><EventID>1000</EventID><TimeCreated SystemTime='2026-10-28T16:42:16.6462563Z'/></System><EventData><Data Name='AppName'>ProjectZomboid64.exe</Data><Data Name='ExceptionCode'>c0000409</Data></EventData></Event><Event xmlns='x'><System><Provider Name='Application Error'/><EventID>1000</EventID><TimeCreated SystemTime='2026-10-28T16:42:16.0000000Z'/></System><EventData><Data Name='AppName'>opera.exe</Data></EventData></Event>";

    #[test]
    fn the_event_says_what_held_the_memory_and_whether_the_game_died() {
        let since = chrono::DateTime::parse_from_rfc3339("2026-10-01T00:00:00Z").unwrap().timestamp();
        let events = events_from(SYSTEM, APP, since);
        assert_eq!(events.len(), 1, "the September one is before `since`");
        let e = &events[0];
        assert!((e.commit_limit_gb - 61.5).abs() < 0.1 && (e.commit_charge_gb - 61.5).abs() < 0.1);
        assert!((e.physical_gb - 31.8).abs() < 0.1);
        assert!((e.game_gb.unwrap() - 24.6).abs() < 0.1);
        assert_eq!(e.others.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), vec!["opera.exe", "Code.exe"], "two opera processes are one line, a nameless one is none");
        assert!((e.others[0].gb - 4.0).abs() < 0.01);
        assert!(e.game_crashed);
        assert_eq!(parse_game_crashes(APP).len(), 1, "only the game's own crash counts");
    }
}

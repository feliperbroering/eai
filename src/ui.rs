use std::time::Duration;

use console::{Term, measure_text_width, style};

// ── terminal input flush ──────────────────────────────────────────────────

/// Flush any pending bytes in the terminal input buffer (e.g. leftover Enter
/// from dialoguer). Prevents key-presses from leaking into the next read_key().
#[cfg(unix)]
pub fn flush_stdin() {
    unsafe extern "C" {
        fn tcflush(fd: i32, action: i32) -> i32;
    }
    const TCIFLUSH: i32 = 1;
    unsafe { tcflush(0, TCIFLUSH) };
}

#[cfg(not(unix))]
pub fn flush_stdin() {}

use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};

// ── brand ──────────────────────────────────────────────────────────────────

const LOGO: &[&str] = &[
    r"  ███████╗ █████╗ ██╗",
    r"  ██╔════╝██╔══██╗██║",
    r"  █████╗  ███████║██║",
    r"  ██╔══╝  ██╔══██║██║",
    r"  ███████╗██║  ██║██║",
    r"  ╚══════╝╚═╝  ╚═╝╚═╝",
];

const MASCOT: &[&str] = &["    ╭─────╮", "    │ >_  │", "    ╰─────╯"];

const TAGLINE: &str = "don't memorize 1000 flags. just prompt it.";

const GREETINGS: &[&str] = &[
    "what are we breaking today?",
    "flags are for golf courses.",
    "your shell, but make it vibes.",
    "the AI that reads man pages for you.",
    "who needs aliases when you have eai?",
    "because life's too short for --help.",
    "proudly mass-producing one-liners since 2025.",
    "turning coffee into commands since boot.",
    "the intern that actually reads docs.",
    "like autocomplete, but it gets you.",
];

const SPINNER_MESSAGES: &[&str] = &[
    "consulting the oracle...",
    "decoding your intent...",
    "translating human to shell...",
    "asking the machines nicely...",
    "crafting the perfect incantation...",
    "parsing vibes...",
    "converting caffeine to commands...",
    "summoning the right flags...",
    "reading the man pages so you don't have to...",
    "doing the thing you hate doing...",
    "brb, checking stack overflow...",
    "compiling your thoughts...",
    "turning english into bash...",
    "flag negotiation in progress...",
    "almost there, probably...",
    "sipping coffee while thinking...",
    "arguing with the kernel...",
    "consulting ancient scrolls (man pages)...",
    "reverse-engineering your brain...",
    "this is easier than remembering tar flags...",
    "definitely faster than googling it...",
    "hallucinating responsibly...",
    "making bash look easy since 2025...",
    "sudo make me a sandwich...",
    "rm -rf doubt...",
    "404 flags not memorized...",
    "piping your dreams to reality...",
    "chmod +x your_idea...",
    "grepping the meaning of life...",
    "it's not a bug, it's a prompt...",
];

const FAILURE_MESSAGES: &[&str] = &[
    "oof. that didn't land.",
    "the shell has spoken. it said no.",
    "not great, not terrible.",
    "segfault in confidence.",
    "let's pretend that didn't happen.",
    "have you tried turning it off and on again?",
    "error: success not found.",
];

// ── color primitives ───────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Rgb(u8, u8, u8);

impl Rgb {
    const CYAN: Self = Self(0, 212, 255);
    const PURPLE: Self = Self(168, 85, 247);
    const PINK: Self = Self(236, 72, 153);
    const GREEN: Self = Self(74, 222, 128);
    const YELLOW: Self = Self(250, 204, 21);
    const BORDER: Self = Self(75, 85, 99);
    const DIM: Self = Self(120, 113, 108);
}

fn lerp(from: &Rgb, to: &Rgb, t: f64) -> Rgb {
    Rgb(
        (from.0 as f64 + (to.0 as f64 - from.0 as f64) * t) as u8,
        (from.1 as f64 + (to.1 as f64 - from.1 as f64) * t) as u8,
        (from.2 as f64 + (to.2 as f64 - from.2 as f64) * t) as u8,
    )
}

fn fg(text: &str, c: &Rgb) -> String {
    if !console::colors_enabled_stderr() {
        return text.to_string();
    }
    format!("\x1b[38;2;{};{};{}m{}\x1b[0m", c.0, c.1, c.2, text)
}

fn fg_bold(text: &str, c: &Rgb) -> String {
    if !console::colors_enabled_stderr() {
        return text.to_string();
    }
    format!("\x1b[1;38;2;{};{};{}m{}\x1b[0m", c.0, c.1, c.2, text)
}

fn gradient(text: &str, from: &Rgb, to: &Rgb) -> String {
    if !console::colors_enabled_stderr() {
        return text.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len().max(1);
    let mut out = String::new();
    for (i, ch) in chars.iter().enumerate() {
        let t = if len > 1 {
            i as f64 / (len - 1) as f64
        } else {
            0.0
        };
        let c = lerp(from, to, t);
        out.push_str(&format!("\x1b[1;38;2;{};{};{}m{}", c.0, c.1, c.2, ch));
    }
    out.push_str("\x1b[0m");
    out
}

// ── banner ─────────────────────────────────────────────────────────────────

pub fn banner() {
    let line_count = LOGO.len();

    eprintln!();
    for (i, logo_line) in LOGO.iter().enumerate() {
        let t = if line_count > 1 {
            i as f64 / (line_count - 1) as f64
        } else {
            0.0
        };
        let color = lerp(&Rgb::CYAN, &Rgb::PURPLE, t);

        let mascot_part = if i < MASCOT.len() {
            fg(MASCOT[i], &Rgb::DIM)
        } else {
            String::new()
        };

        eprintln!("{}{}", fg_bold(logo_line, &color), mascot_part);
        std::thread::sleep(Duration::from_millis(25));
    }

    eprintln!();
    eprintln!("  {}", gradient(TAGLINE, &Rgb::CYAN, &Rgb::PINK));
    eprintln!("  {}", fg(random_pick(GREETINGS), &Rgb::DIM));
    eprintln!("  {}", fg(&"━".repeat(46), &Rgb::BORDER));
    eprintln!();
}

// ── command display ────────────────────────────────────────────────────────

pub fn print_command(command: &str, explanation: Option<&str>) {
    let label = format!("❯ {}", command);
    let width = measure_text_width(&label);
    let pad = 3;
    let inner = width + pad * 2;
    let total_box_width = inner + 4;

    let term_width = Term::stderr().size().1 as usize;
    let use_box = total_box_width <= term_width;

    let styled_cmd = format!(
        "{} {}",
        fg_bold("❯", &Rgb::CYAN),
        style(command).white().bold()
    );

    eprintln!();
    if use_box {
        eprintln!(
            "  {}",
            gradient(
                &format!("╭{}╮", "─".repeat(inner)),
                &Rgb::CYAN,
                &Rgb::PURPLE
            )
        );
        eprintln!(
            "  {}{}{}{}{}",
            fg("│", &Rgb::CYAN),
            " ".repeat(pad),
            styled_cmd,
            " ".repeat(pad),
            fg("│", &Rgb::PURPLE),
        );
        eprintln!(
            "  {}",
            gradient(
                &format!("╰{}╯", "─".repeat(inner)),
                &Rgb::PURPLE,
                &Rgb::CYAN
            )
        );
    } else {
        eprintln!(
            "  {} {}",
            gradient("━━━", &Rgb::CYAN, &Rgb::PURPLE),
            styled_cmd
        );
    }

    if let Some(explain) = explanation {
        eprintln!(
            "  {} {}",
            fg("//", &Rgb::DIM),
            style(explain).dim().italic()
        );
    }
    eprintln!();
}

// ── execution status ───────────────────────────────────────────────────────

pub fn print_failure(exit_code: i32) {
    let msg = random_pick(FAILURE_MESSAGES);
    eprintln!(
        "  {} {} {}",
        style("✗").red().bold(),
        style(format!("exit {exit_code}")).red(),
        style(format!("— {msg}")).red().dim()
    );
}

pub fn print_empty_output() {
    eprintln!(
        "  {} {}",
        style("∅").yellow().bold(),
        style("command ran but produced no output").yellow().dim()
    );
}

// ── spinners ───────────────────────────────────────────────────────────────

const BRAILLE_TICKS: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::with_draw_target(None, ProgressDrawTarget::stderr());
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(BRAILLE_TICKS)
            .template("  {spinner:.cyan} {msg:.dim}")
            .expect("valid template"),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

pub fn generation_spinner(backend_label: &str) -> ProgressBar {
    let msg = random_pick(SPINNER_MESSAGES);
    let pb = ProgressBar::with_draw_target(None, ProgressDrawTarget::stderr());
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(BRAILLE_TICKS)
            .template("  {spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    pb.set_message(format!(
        "{} {}",
        style(msg).dim(),
        style(format!("[{backend_label}]")).dim().italic()
    ));
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

fn random_pick<'a>(items: &'a [&'a str]) -> &'a str {
    use std::time::SystemTime;
    let seed = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize;
    items[seed % items.len()]
}

// ── status messages ────────────────────────────────────────────────────────

pub fn status_ok(msg: &str) {
    eprintln!("  {} {}", style("✓").green().bold(), style(msg).dim());
}

pub fn status_warn(msg: &str) {
    eprintln!("  {} {}", style("⚠").yellow().bold(), style(msg).yellow());
}

// ── action menus ───────────────────────────────────────────────────────────

pub enum PreAction {
    Execute,
    Edit,
    Refine,
    Search,
    Cancel,
}

pub enum PostAction {
    Refine,
    Search,
    Quit,
}

struct ActionItem {
    label: &'static str,
    key: &'static str,
    color: Rgb,
}

const PRE_ACTIONS: &[ActionItem] = &[
    ActionItem {
        label: "run",
        key: "↵",
        color: Rgb::GREEN,
    },
    ActionItem {
        label: "edit",
        key: "e",
        color: Rgb::CYAN,
    },
    ActionItem {
        label: "refine",
        key: "r",
        color: Rgb::PURPLE,
    },
    ActionItem {
        label: "search",
        key: "s",
        color: Rgb::YELLOW,
    },
    ActionItem {
        label: "cancel",
        key: "^C",
        color: Rgb::DIM,
    },
];

const POST_ACTIONS: &[ActionItem] = &[
    ActionItem {
        label: "refine",
        key: "r",
        color: Rgb::PURPLE,
    },
    ActionItem {
        label: "search",
        key: "s",
        color: Rgb::YELLOW,
    },
    ActionItem {
        label: "quit",
        key: "q",
        color: Rgb::DIM,
    },
];

pub fn prompt_before_execution(
    command: &str,
    explanation: Option<&str>,
) -> anyhow::Result<PreAction> {
    print_action_bar(PRE_ACTIONS);

    let label = format!("❯ {}", command);
    let width = measure_text_width(&label);
    let pad = 3;
    let total_box_width = width + pad * 2 + 4;
    let term_width = Term::stderr().size().1 as usize;
    let use_box = total_box_width <= term_width;

    let box_lines = if use_box { 3 } else { 1 };
    let explain_lines = if explanation.is_some() { 1 } else { 0 };
    let total_clear = 1 + box_lines + explain_lines + 1 + 3;

    let term = Term::stdout();
    loop {
        match term.read_key()? {
            console::Key::Enter => return Ok(PreAction::Execute),
            console::Key::Char('e' | 'E') => return Ok(PreAction::Edit),
            console::Key::Char('r' | 'R') => return Ok(PreAction::Refine),
            console::Key::Char('s' | 'S') => return Ok(PreAction::Search),
            console::Key::CtrlC => return Ok(PreAction::Cancel),
            _ => {
                let _ = Term::stderr().clear_last_lines(total_clear);
                print_command(command, explanation);
                print_action_bar(PRE_ACTIONS);
            }
        }
    }
}

pub fn prompt_after_execution() -> anyhow::Result<PostAction> {
    print_action_bar(POST_ACTIONS);

    let term = Term::stdout();
    loop {
        match term.read_key()? {
            console::Key::Char('r' | 'R') => return Ok(PostAction::Refine),
            console::Key::Char('s' | 'S') => return Ok(PostAction::Search),
            console::Key::Char('q' | 'Q') | console::Key::CtrlC => return Ok(PostAction::Quit),
            _ => {}
        }
    }
}

fn print_action_bar(items: &[ActionItem]) {
    let parts: Vec<String> = items
        .iter()
        .map(|item| {
            format!(
                "{} {} {}",
                fg("●", &item.color),
                item.label,
                fg_bold(item.key, &item.color),
            )
        })
        .collect();

    let sep = format!("  {}  ", fg("│", &Rgb::BORDER));
    let content = parts.join(&sep);
    let width = measure_text_width(&content);
    let pad = 2;
    let inner = width + pad * 2;

    eprintln!(
        "  {}",
        fg(&format!("╭{}╮", "─".repeat(inner)), &Rgb::BORDER)
    );
    eprintln!(
        "  {}{}{}{}{}",
        fg("│", &Rgb::BORDER),
        " ".repeat(pad),
        content,
        " ".repeat(pad),
        fg("│", &Rgb::BORDER),
    );
    eprintln!(
        "  {}",
        fg(&format!("╰{}╯", "─".repeat(inner)), &Rgb::BORDER)
    );
}

// ── explanation display ────────────────────────────────────────────────────

pub fn print_explanation(text: &str) {
    eprintln!();
    for line in text.lines() {
        if line.trim().is_empty() {
            eprintln!();
        } else {
            eprintln!("  {}", style(line).white());
        }
    }
    eprintln!();
}

pub fn print_stdin_badge(size: usize) {
    let human = if size > 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else if size > 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else {
        format!("{} bytes", size)
    };
    eprintln!(
        "  {} {}",
        fg("⏐", &Rgb::CYAN),
        style(format!("piped {human} from stdin")).dim()
    );
}

// ── tool discovery ──────────────────────────────────────────────────────────

fn format_stars(n: u64) -> String {
    if n >= 1000 {
        format!("{:.1}k", n as f64 / 1000.0)
    } else {
        n.to_string()
    }
}

pub fn print_tool_suggestions(suggestions: &[crate::tool_context::ToolSuggestion]) {
    eprintln!();
    eprintln!(
        "  {} {}",
        fg_bold("🔍", &Rgb::CYAN),
        style("Tools found for your task:").white().bold()
    );
    eprintln!();

    for (i, s) in suggestions.iter().enumerate() {
        let mut meta_parts = Vec::new();
        if let Some(stars) = s.stars {
            meta_parts.push(format!("★ {}", format_stars(stars)));
        }
        if let Some(contribs) = s.contributors {
            if contribs > 1 {
                meta_parts.push(format!("{contribs} contributors"));
            }
        }
        if let Some(ref ver) = s.version {
            meta_parts.push(ver.clone());
        }

        let meta = if meta_parts.is_empty() {
            String::new()
        } else {
            format!("  {}", style(meta_parts.join(" · ")).dim())
        };

        eprintln!(
            "  {}  {}{}",
            fg_bold(&format!("{}.", i + 1), &Rgb::CYAN),
            style(&s.name).white().bold(),
            meta,
        );
        eprintln!("     {}", style(&s.description).dim().italic());
        if let Some(ref review) = s.review {
            eprintln!("     {}", fg(review, &Rgb::GREEN));
        }
        eprintln!("     {}", fg(&s.repo_url, &Rgb::CYAN));
        eprintln!("     {}", style(&s.install_cmd).dim());
        eprintln!();
    }
}

pub enum InstallAction {
    Install(usize),
    Skip,
    Cancel,
}

pub fn prompt_tool_install(count: usize) -> anyhow::Result<InstallAction> {
    let range = if count == 1 {
        "1".to_string()
    } else {
        format!("1-{count}")
    };

    let parts: Vec<String> = [
        (format!("install {range}"), Rgb::GREEN),
        ("skip s".to_string(), Rgb::YELLOW),
        ("cancel ^C".to_string(), Rgb::DIM),
    ]
    .iter()
    .map(|(text, color)| {
        let mut words = text.splitn(2, ' ');
        let label = words.next().unwrap_or("");
        let key = words.next().unwrap_or("");
        format!("{} {} {}", fg("●", color), label, fg_bold(key, color))
    })
    .collect();

    let sep = format!("  {}  ", fg("│", &Rgb::BORDER));
    let content = parts.join(&sep);
    let width = measure_text_width(&content);
    let pad = 2;
    let inner = width + pad * 2;

    eprintln!(
        "  {}",
        fg(&format!("╭{}╮", "─".repeat(inner)), &Rgb::BORDER)
    );
    eprintln!(
        "  {}{}{}{}{}",
        fg("│", &Rgb::BORDER),
        " ".repeat(pad),
        content,
        " ".repeat(pad),
        fg("│", &Rgb::BORDER)
    );
    eprintln!(
        "  {}",
        fg(&format!("╰{}╯", "─".repeat(inner)), &Rgb::BORDER)
    );

    let term = Term::stdout();
    loop {
        match term.read_key()? {
            console::Key::Char(c) if c.is_ascii_digit() => {
                let idx = (c as u8 - b'0') as usize;
                if idx >= 1 && idx <= count {
                    return Ok(InstallAction::Install(idx - 1));
                }
            }
            console::Key::Char('s' | 'S') => return Ok(InstallAction::Skip),
            console::Key::CtrlC => return Ok(InstallAction::Cancel),
            _ => {}
        }
    }
}

// ── update notification ───────────────────────────────────────────────────

pub fn print_update_available(current: &str, latest: &str) {
    eprintln!(
        "  {} {}  {} → {}",
        fg_bold("⬆", &Rgb::GREEN),
        style("new version available!").white().bold(),
        style(current).dim(),
        fg_bold(latest, &Rgb::GREEN),
    );
    eprintln!("  {}", fg("  update now? (Y/n)", &Rgb::DIM),);
}

pub fn print_update_success(version: &str) {
    eprintln!(
        "  {} {}",
        style("✓").green().bold(),
        style(format!("updated to {version}!")).dim(),
    );
}

// ── input prompts ──────────────────────────────────────────────────────────

pub fn prompt_text(prompt: &str, initial: &str) -> anyhow::Result<String> {
    let styled_prompt = format!("{} {}", style("›").cyan(), style(prompt).bold());
    let mut input = dialoguer::Input::<String>::new().with_prompt(styled_prompt);
    if !initial.is_empty() {
        input = input.with_initial_text(initial.to_string());
    }
    Ok(input.allow_empty(true).interact_text()?)
}

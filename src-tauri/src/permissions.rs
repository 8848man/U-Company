//! ADR-0058: command permissions a teammate needs, approved by the CEO inside one goal conversation.
//! A provider reports denied calls; this module turns them (and the commands a plan names) into
//! rules, classifies each rule, and keeps the conversation's grants. Nothing here runs a command.

use crate::model::{Goal, GoalGuidance};
use serde::{Deserialize, Serialize};

/// A call the provider refused because nobody could approve it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Denial { pub tool: String, pub command: String }

/// `safe`: included in `모두 승인`. `individual`: needs its own button. `blocked`: the app never grants it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Rule { pub tool: String, pub prefix: String, pub class: String, pub reason: String }

impl Rule {
    pub fn key(&self) -> String { if self.prefix.is_empty() { self.tool.clone() } else { format!("{}({})", self.tool, self.prefix) } }
    /// What the Claude CLI gets: the bare command and the command with arguments.
    pub fn cli_rules(&self) -> Vec<String> {
        if self.prefix.is_empty() { return vec![self.tool.clone()]; }
        vec![format!("{}({})", self.tool, self.prefix), format!("{}({} *)", self.tool, self.prefix)]
    }
}

/// A request shown on a card: rules proposed for a plan (`plan`) or after a blocked call (`blocked`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandRequest {
    pub id: String,
    pub goal_id: String,
    pub conversation_id: String,
    pub source: String,
    pub rules: Vec<Rule>,
    #[serde(default)]
    pub granted: Vec<String>,
    #[serde(default)]
    pub task_ids: Vec<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CommandGrants { pub conversation_id: String, #[serde(default)] pub rules: Vec<Rule> }

pub const OPERATION: &str = "command_permission";
pub const MAX_RESUMES: u32 = 2;

const SHELLS: [&str; 2] = ["Bash", "PowerShell"];
/// Commands that delete, leave the project, publish, use the network, change rights or start another shell.
const INDIVIDUAL: [&str; 34] = ["rm", "del", "erase", "rmdir", "rd", "mv", "move", "curl", "wget", "invoke-webrequest", "iwr", "irm", "invoke-restmethod", "ssh", "scp", "sftp", "sudo", "su", "chmod", "chown", "icacls", "takeown", "powershell", "pwsh", "cmd", "bash", "sh", "zsh", "start", "reg", "taskkill", "kill", "npx", "format"];
const INDIVIDUAL_SUB: [(&str, &[&str]); 7] = [
    ("git", &["push", "remote", "reset", "clean", "rebase", "filter-branch", "config", "checkout", "restore", "rm", "gc"]),
    ("npm", &["publish", "install", "i", "add", "uninstall", "exec", "login", "adduser", "ci", "update"]),
    ("pnpm", &["publish", "install", "i", "add", "remove", "dlx", "update"]),
    ("yarn", &["publish", "add", "remove", "install", "dlx", "up"]),
    ("pip", &["install", "uninstall", "download"]),
    ("cargo", &["publish", "install", "login"]),
    ("gh", &["pr", "release", "repo", "api", "auth"]),
];

/// ADR-0063: built-in read-only commands. Claude Code decides whether they run; the app never makes
/// a rule or grant for them, and they do not stop a later line from counting as covered.
const READ_ONLY: [&str; 29] = ["cd", "ls", "dir", "pwd", "echo", "printf", "cat", "type", "head", "tail", "grep", "findstr", "wc", "which", "where", "diff", "stat", "du", "true", "false", "cut", "sort", "uniq", "tr", "basename", "dirname", "realpath", "test", "["];

/// Shell words that only open or close a loop or condition. `do`/`then`/`else` and the like are
/// dropped from the front of a command; `if`/`while`/`until`/`elif` are dropped and their condition
/// is judged as the command it is. `for …`, `case …` and `select …` headers run nothing.
const LEADING_KEYWORDS: [&str; 11] = ["do", "then", "else", "elif", "if", "while", "until", "{", "(", "!", "time"];
const CLOSING_KEYWORDS: [&str; 5] = ["done", "fi", "esac", "}", ")"];
const HEADERS: [&str; 3] = ["for", "case", "select"];

/// ADR-0063: the command a split part actually runs, without shell loop/condition words.
/// `None` when the part runs no command (`done`, `for f in *.ts`).
pub fn command_part(part: &str) -> Option<String> {
    let mut words = raw_words(part);
    while words.first().is_some_and(|word| LEADING_KEYWORDS.contains(&word.as_str())) { words.remove(0); }
    let first = words.first()?;
    if CLOSING_KEYWORDS.contains(&first.as_str()) || HEADERS.contains(&first.as_str()) { return None; }
    Some(words.join(" "))
}

/// The commands of one shell line, split and without loop/condition words.
fn commands_of(line: &str) -> Vec<String> { split_commands(line).iter().filter_map(|part| command_part(part)).collect() }

/// Whether one command (already split from its line) is a built-in read-only command.
pub fn is_read_only(command: &str) -> bool {
    let Some(command) = command_part(command) else { return true };
    words(&command).first().is_some_and(|first| !first.contains('=') && READ_ONLY.contains(&base_name(first).as_str()))
}

fn base_name(token: &str) -> String {
    let name = token.rsplit(['/', '\\']).next().unwrap_or(token).to_ascii_lowercase();
    name.strip_suffix(".exe").or_else(|| name.strip_suffix(".cmd")).map(str::to_string).unwrap_or(name)
}

/// Splits a shell line into its commands at `;`, `&&`, `||`, `|` and line breaks (quotes respected).
pub fn split_commands(line: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match quote {
            Some(q) if c == q => { quote = None; current.push(c); }
            Some(_) => current.push(c),
            None if c == '"' || c == '\'' => { quote = Some(c); current.push(c); }
            None if c == ';' || c == '\n' || c == '|' || (c == '&' && chars.get(i + 1) == Some(&'&')) => {
                if (c == '|' || c == '&') && chars.get(i + 1) == Some(&c) { i += 1; }
                if !current.trim().is_empty() { parts.push(current.trim().to_string()); }
                current.clear();
            }
            None => current.push(c),
        }
        i += 1;
    }
    if !current.trim().is_empty() { parts.push(current.trim().to_string()); }
    parts
}

fn words(command: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for c in command.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => current.push(c),
            None if c == '"' || c == '\'' => quote = Some(c),
            None if c.is_whitespace() => { if !current.is_empty() { out.push(std::mem::take(&mut current)); } }
            None => current.push(c),
        }
    }
    if !current.is_empty() { out.push(current); }
    out
}

/// The raw text of each word (quotes kept), so a rule matches the command as the shell sees it.
fn raw_words(command: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    for c in command.chars() {
        match quote {
            Some(q) if c == q => { quote = None; current.push(c); }
            Some(_) => current.push(c),
            None if c == '"' || c == '\'' => { quote = Some(c); current.push(c); }
            None if c.is_whitespace() => { if !current.is_empty() { out.push(std::mem::take(&mut current)); } }
            None => current.push(c),
        }
    }
    if !current.is_empty() { out.push(current); }
    out
}

/// One command → the prefix a rule allows: the program and, when present, its first argument
/// (`npm test`, `node --test`, `node a.js`). Redirections end the prefix.
fn prefix_of(command: &str) -> String {
    let words: Vec<String> = raw_words(command).into_iter().take_while(|word| !word.starts_with('>') && !word.starts_with('<') && !word.starts_with("2>")).collect();
    words.into_iter().take(2).collect::<Vec<_>>().join(" ")
}

pub fn classify(tool: &str, prefix: &str) -> (String, String) {
    if !SHELLS.contains(&tool) {
        return match tool {
            "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => ("blocked".into(), "작업 폴더 밖 파일 쓰기는 앱이 허용하지 않아요".into()),
            "WebFetch" | "WebSearch" => ("individual".into(), "인터넷에 접속해요".into()),
            _ => ("individual".into(), "명령이 아닌 도구라 따로 확인해요".into()),
        };
    }
    let words = words(prefix);
    let Some(first) = words.first() else { return ("blocked".into(), "모든 명령을 허용하는 규칙은 만들 수 없어요".into()) };
    if first == "*" || first.contains('*') { return ("blocked".into(), "모든 명령을 허용하는 규칙은 만들 수 없어요".into()); }
    if first.contains('=') { return ("individual".into(), "환경 변수를 바꿔 실행해요".into()); }
    if prefix.contains("..") || prefix.contains('~') || words.iter().skip(1).any(|word| word.starts_with('/') || word.contains(":\\") || word.contains(":/")) {
        return ("individual".into(), "작업 폴더 밖 경로를 가리켜요".into());
    }
    let program = base_name(first);
    if INDIVIDUAL.contains(&program.as_str()) { return ("individual".into(), reason_for(&program).into()); }
    if let Some((_, subs)) = INDIVIDUAL_SUB.iter().find(|(name, _)| *name == program) {
        if let Some(sub) = words.get(1).map(|word| word.to_ascii_lowercase()) {
            if subs.contains(&sub.as_str()) { return ("individual".into(), format!("{program} {sub}: 설치·배포·저장소 변경처럼 작업 폴더 밖에 영향을 줄 수 있어요")); }
        }
    }
    ("safe".into(), String::new())
}

fn reason_for(program: &str) -> &'static str {
    match program {
        "rm" | "del" | "erase" | "rmdir" | "rd" => "파일을 지워요",
        "mv" | "move" => "파일을 옮겨요",
        "curl" | "wget" | "invoke-webrequest" | "iwr" | "irm" | "invoke-restmethod" | "ssh" | "scp" | "sftp" | "npx" => "인터넷에 접속하거나 내려받아요",
        "sudo" | "su" | "chmod" | "chown" | "icacls" | "takeown" => "권한을 바꿔요",
        "powershell" | "pwsh" | "cmd" | "bash" | "sh" | "zsh" | "start" => "다른 셸이나 프로그램을 실행해요",
        _ => "시스템을 바꿀 수 있어요",
    }
}

fn rule(tool: &str, prefix: &str) -> Rule {
    let (class, reason) = classify(tool, prefix);
    Rule { tool: tool.into(), prefix: prefix.into(), class, reason }
}

fn push_unique(rules: &mut Vec<Rule>, candidate: Rule) {
    if !rules.iter().any(|existing| existing.key().eq_ignore_ascii_case(&candidate.key())) { rules.push(candidate); }
}

/// Rules that would let the denied calls run, one per command in a chained line.
pub fn rules_for_denials(denials: &[Denial]) -> Vec<Rule> {
    let mut rules = Vec::new();
    for denial in denials {
        if SHELLS.contains(&denial.tool.as_str()) {
            for command in commands_of(&denial.command).into_iter().filter(|command| !is_read_only(command)) { let prefix = prefix_of(&command); push_unique(&mut rules, rule(&denial.tool, &prefix)); }
        } else {
            push_unique(&mut rules, rule(&denial.tool, ""));
        }
    }
    rules
}

/// Rules for the commands a plan's tasks say they will run.
pub fn rules_for_commands(commands: &[String]) -> Vec<Rule> {
    let mut rules = Vec::new();
    for line in commands { for command in commands_of(line).into_iter().filter(|command| !is_read_only(command)) { push_unique(&mut rules, rule("Bash", &prefix_of(&command))); } }
    rules
}

/// Whether a granted rule already covers a denied command (so it is not asked again). Read-only
/// parts are ignored (ADR-0063); a line made only of read-only parts is never covered, because no
/// grant can explain why it was refused.
pub fn covered(grants: &[Rule], tool: &str, command: &str) -> bool {
    let commands: Vec<String> = if SHELLS.contains(&tool) { commands_of(command).into_iter().filter(|part| !is_read_only(part)).collect() } else { vec![String::new()] };
    if commands.is_empty() { return false; }
    commands.iter().all(|part| {
        let prefix = if SHELLS.contains(&tool) { prefix_of(part) } else { String::new() };
        grants.iter().any(|grant| grant.tool == tool && (grant.prefix.is_empty() && prefix.is_empty() || !grant.prefix.is_empty() && (prefix == grant.prefix || prefix.starts_with(&format!("{} ", grant.prefix)))))
    })
}

/// The first goal of a version chain: grants belong to the whole visible conversation.
pub fn conversation_id(goal_id: &str, goals: &[Goal], guidance: &[GoalGuidance]) -> String {
    let mut current = goal_id.to_string();
    for _ in 0..64 {
        match guidance.iter().find(|item| item.replacement_goal_id.as_deref() == Some(current.as_str())) {
            Some(item) if goals.iter().any(|goal| goal.id == item.goal_id) => current = item.goal_id.clone(),
            _ => break,
        }
    }
    current
}

/// Validates the CEO's choice against a request: only listed, grantable rules.
pub fn chosen_rules(request: &CommandRequest, keys: &[String]) -> Result<Vec<Rule>, String> {
    let mut chosen = Vec::new();
    for key in keys {
        let rule = request.rules.iter().find(|rule| rule.key() == *key).ok_or_else(|| format!("요청에 없는 권한입니다: {key}"))?;
        if rule.class == "blocked" { return Err(format!("앱이 허용하지 않는 권한입니다: {key}")); }
        chosen.push(rule.clone());
    }
    Ok(chosen)
}

/// The Claude `--allowedTools` values for a conversation's grants.
pub fn cli_allowed(grants: &[Rule]) -> Vec<String> { grants.iter().flat_map(Rule::cli_rules).collect() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chained_commands_become_one_rule_per_command() {
        assert_eq!(split_commands(r#"node --test; echo "=== exit: $? ===" && npm test || true | tee "a;b.txt""#), vec!["node --test", r#"echo "=== exit: $? ===""#, "npm test", "true", r#"tee "a;b.txt""#]);
        let rules = rules_for_denials(&[Denial { tool: "Bash".into(), command: "node --test tests/source.test.mjs; echo done".into() }, Denial { tool: "Bash".into(), command: "node --test".into() }]);
        assert_eq!(rules.iter().map(Rule::key).collect::<Vec<_>>(), vec!["Bash(node --test)"], "read-only echo makes no rule (ADR-0063)");
        assert!(rules.iter().all(|rule| rule.class == "safe"));
        assert_eq!(rules[0].cli_rules(), vec!["Bash(node --test)", "Bash(node --test *)"]);
        assert_eq!(prefix_of("npm test > out.txt 2>&1"), "npm test");
    }

    #[test]
    fn risky_commands_need_their_own_approval_and_some_are_never_granted() {
        for (tool, prefix, class) in [
            ("Bash", "npm test", "safe"), ("Bash", "node --test", "safe"), ("Bash", "git status", "safe"), ("Bash", "cargo test", "safe"),
            ("Bash", "rm -rf", "individual"), ("Bash", "del README.md", "individual"), ("Bash", "curl https://x", "individual"), ("Bash", "git push", "individual"),
            ("Bash", "npm install", "individual"), ("Bash", "npm publish", "individual"), ("Bash", "powershell -c", "individual"), ("Bash", "C:\\tools\\rm.exe x", "individual"),
            ("Bash", "cat ../secret", "individual"), ("Bash", "FOO=1 npm", "individual"), ("WebFetch", "", "individual"),
            ("Bash", "", "blocked"), ("Bash", "*", "blocked"), ("Write", "", "blocked"),
        ] { assert_eq!(classify(tool, prefix).0, class, "{tool}({prefix})"); }
    }

    #[test]
    fn grants_cover_later_calls_by_prefix_and_choices_are_checked() {
        let grants = rules_for_commands(&["node --test".into(), "npm test".into()]);
        assert!(covered(&grants, "Bash", "node --test tests/a.mjs"));
        assert!(covered(&grants, "Bash", "npm test && node --test"));
        assert!(!covered(&grants, "Bash", "npm test; node b.js"));
        assert!(!covered(&grants, "Bash", "node --testx"));
        let request = CommandRequest { id: "a".into(), goal_id: "g".into(), conversation_id: "g".into(), source: "blocked".into(), rules: rules_for_denials(&[Denial { tool: "Bash".into(), command: "npm test; rm -rf dist".into() }, Denial { tool: "Write".into(), command: String::new() }]), granted: vec![], task_ids: vec![], created_at: "t".into() };
        assert_eq!(chosen_rules(&request, &["Bash(npm test)".into(), "Bash(rm -rf)".into()]).unwrap().len(), 2, "an individual rule can be chosen explicitly");
        assert!(chosen_rules(&request, &["Write".into()]).is_err());
        assert!(chosen_rules(&request, &["Bash(curl x)".into()]).is_err());
    }

    #[test]
    fn read_only_commands_make_no_rule_and_do_not_block_coverage() {
        assert!(is_read_only("cd src") && is_read_only(r"cd C:\work\x") && is_read_only("cd ..") && is_read_only("ls -la") && is_read_only("echo done"));
        assert!(!is_read_only("npm test") && !is_read_only("FOO=1 cd x") && !is_read_only("find . -delete") && !is_read_only("rm -rf x"));
        let rules = rules_for_denials(&[Denial { tool: "Bash".into(), command: r"cd C:\Users\me\task && npm test; echo done".into() }]);
        assert_eq!(rules.iter().map(Rule::key).collect::<Vec<_>>(), vec!["Bash(npm test)"], "cd and echo never reach the card");
        assert_eq!(rules_for_commands(&["cd app && cargo test".into()]).iter().map(Rule::key).collect::<Vec<_>>(), vec!["Bash(cargo test)"]);
        let grants = rules_for_commands(&["npm test".into()]);
        assert!(covered(&grants, "Bash", r"cd C:\other && npm test"), "a different cd does not bring the card back");
        assert!(!covered(&grants, "Bash", "cd .. && ls"), "a line of only read-only commands is never covered");
        assert!(rules_for_denials(&[Denial { tool: "Bash".into(), command: "cd ../outside".into() }]).is_empty());
    }

    #[test]
    fn shell_loops_and_conditions_only_ask_for_the_commands_they_run() {
        // The card a CEO saw on 2026-11-01: a loop of grep/cut/echo around one test command.
        let line = r#"for f in src/*.ts; do echo "== $f"; grep -on "useEffect" "$f" | cut -c1-30; grep -vn "^$" "$f" | head -3; done; npx vitest run"#;
        let rules = rules_for_denials(&[Denial { tool: "Bash".into(), command: line.into() }]);
        assert_eq!(rules.iter().map(Rule::key).collect::<Vec<_>>(), vec!["Bash(npx vitest)"]);
        assert_eq!(command_part("do npm test"), Some("npm test".into()));
        assert_eq!(command_part("if npm test"), Some("npm test".into()));
        assert_eq!(command_part("done"), None);
        assert_eq!(command_part("for f in *.ts"), None);
        let rules = rules_for_denials(&[Denial { tool: "Bash".into(), command: "if [ -f package.json ]; then npm test; else cargo test; fi".into() }]);
        assert_eq!(rules.iter().map(Rule::key).collect::<Vec<_>>(), vec!["Bash(npm test)", "Bash(cargo test)"]);
        let rules = rules_for_denials(&[Denial { tool: "Bash".into(), command: "while true; do rm -rf dist; done".into() }]);
        assert_eq!(rules.iter().map(|rule| (rule.key(), rule.class.clone())).collect::<Vec<_>>(), vec![("Bash(rm -rf)".to_string(), "individual".to_string())], "a risky command inside a loop is still asked about");
        let grants = rules_for_commands(&["npx vitest".into()]);
        assert!(covered(&grants, "Bash", line), "once the test command is granted the whole loop is covered");
    }

    #[test]
    fn grants_belong_to_the_first_goal_of_the_conversation() {
        let goal = |id: &str| -> Goal { serde_json::from_value(serde_json::json!({ "id": id, "intent": "x", "status": "new", "report": "", "createdAt": "t" })).unwrap() };
        let link = |from: &str, to: &str| -> GoalGuidance { serde_json::from_value(serde_json::json!({ "id": format!("{from}{to}"), "goalId": from, "text": "t", "status": "revised", "createdAt": "t", "replacementGoalId": to })).unwrap() };
        let goals = vec![goal("a"), goal("b"), goal("c")];
        let guidance = vec![link("a", "b"), link("b", "c")];
        assert_eq!(conversation_id("c", &goals, &guidance), "a");
        assert_eq!(conversation_id("a", &goals, &guidance), "a");
        assert_eq!(conversation_id("z", &goals, &guidance), "z");
    }
}

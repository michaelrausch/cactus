//! How flow tools reach the person at the terminal. `--var name=value`
//! answers prompts up front, which is how CI runs a flow with a prompt; at
//! a terminal anything else is asked on stderr. Without one, prompts fall
//! back to their defaults.

use std::io::{self, BufRead, IsTerminal, Write};
use std::process::Command;

use cactus_core::env::Env;
use cactus_core::tools::{Answer, Ask, BoxFuture, CHOICE, PromptVar, ToolHost};
use cactus_core::{Error, Result};

pub struct CliHost {
    /// From `--var`.
    pub vars: Env,
    /// Whether stdin is a terminal someone can answer at.
    pub interactive: bool,
}

impl CliHost {
    pub fn new(vars: Env) -> Self {
        Self { vars, interactive: io::stdin().is_terminal() }
    }

    fn answer(&self, ask: &Ask) -> io::Result<Answer> {
        match ask {
            Ask::Fields { title, fields } => {
                let mut answers: Env = fields
                    .iter()
                    .filter_map(|field| {
                        let name = field.name.trim();
                        Some((name.to_string(), self.vars.get(name)?.clone()))
                    })
                    .collect();
                if !self.interactive {
                    return Ok(Answer::Given(answers));
                }
                eprintln!("{title}");
                if fields.is_empty() {
                    let reply = ask_line("Continue? [Y/n] ")?;
                    return Ok(if reply.trim().eq_ignore_ascii_case("n") {
                        Answer::Stopped
                    } else {
                        Answer::Given(answers)
                    });
                }
                for field in fields {
                    let name = field.name.trim();
                    if answers.contains_key(name) {
                        continue;
                    }
                    if let Some(value) = ask_field(field)? {
                        answers.insert(name.to_string(), value);
                    }
                }
                Ok(Answer::Given(answers))
            }
            Ask::Choose { title, choices } => {
                // `--var name=value` picks the item that would set `name` to `value`.
                let matching = choices.iter().position(|choice| {
                    let given: Vec<_> =
                        choice.values.iter().filter_map(|(k, v)| Some((self.vars.get(k)?, v))).collect();
                    !given.is_empty() && given.iter().all(|(wanted, value)| wanted == value)
                });
                if matching.is_some() || !self.interactive {
                    let answer = matching.map(|index| (CHOICE.to_string(), index.to_string()));
                    return Ok(Answer::Given(answer.into_iter().collect()));
                }
                eprintln!("{title}");
                for (number, choice) in choices.iter().enumerate() {
                    eprintln!("  {}. {}", number + 1, choice.label);
                }
                loop {
                    let reply = ask_line(&format!("1-{}: ", choices.len()))?;
                    if reply.trim().is_empty() {
                        return Ok(Answer::Stopped);
                    }
                    let picked = reply.trim().parse::<usize>().ok().and_then(|n| n.checked_sub(1));
                    if let Some(index) = picked.filter(|index| *index < choices.len()) {
                        return Ok(Answer::Given(Env::from([(CHOICE.to_string(), index.to_string())])));
                    }
                }
            }
            Ask::Continue { title, text } => {
                if !self.interactive {
                    return Ok(Answer::Unattended);
                }
                eprintln!("{title}\n{text}");
                ask_line("Press Enter to continue ")?;
                Ok(Answer::Given(Env::new()))
            }
        }
    }
}

/// One value; an empty reply takes the default when there is one.
fn ask_field(field: &PromptVar) -> io::Result<Option<String>> {
    let label = if field.label.trim().is_empty() { field.name.trim() } else { field.label.trim() };
    let default = field.default.as_ref().map(|d| d.trim().to_string());
    let shown = match &default {
        Some(default) if !field.secret => format!("{label} [{default}]: "),
        _ => format!("{label}: "),
    };
    let reply = if field.secret { rpassword::prompt_password(shown)? } else { ask_line(&shown)? };
    Ok(if reply.is_empty() { default } else { Some(reply) })
}

fn ask_line(prompt: &str) -> io::Result<String> {
    eprint!("{prompt}");
    io::stderr().flush()?;
    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    Ok(line.trim_end_matches(['\n', '\r']).to_string())
}

impl ToolHost for CliHost {
    fn ask(&self, ask: Ask) -> BoxFuture<'_, Answer> {
        // Reading the terminal blocks, which is fine: nothing else runs meanwhile.
        let answer = self.answer(&ask).unwrap_or(Answer::Stopped);
        Box::pin(async move { answer })
    }

    // The step's result line prints the text; nothing to add here.
    fn show(&self, _title: &str, _text: &str) {}

    fn notify(&self, title: &str, body: &str) {
        eprintln!("notify: {title}{}", if body.is_empty() { String::new() } else { format!(": {body}") });
    }

    fn open_url(&self, url: &str) -> Result<()> {
        eprintln!("open: {url}");
        // In CI there's no browser to open, so just print it.
        if !self.interactive {
            return Ok(());
        }
        let opener = if cfg!(target_os = "macos") {
            Command::new("open").arg(url).status()
        } else if cfg!(windows) {
            Command::new("cmd").args(["/C", "start", "", url]).status()
        } else {
            Command::new("xdg-open").arg(url).status()
        };
        opener.map(drop).map_err(|e| Error::Tool(format!("Couldn't open {url}: {e}")))
    }
}

/// `name=value`, for `--var`.
pub fn parse_var(text: &str) -> std::result::Result<(String, String), String> {
    match text.split_once('=') {
        Some((name, value)) if !name.trim().is_empty() => Ok((name.trim().to_string(), value.to_string())),
        _ => Err(format!("expected name=value, got {text:?}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vars() {
        assert_eq!(parse_var("code=a=b").unwrap(), ("code".into(), "a=b".into()));
        assert_eq!(parse_var("empty=").unwrap(), ("empty".into(), String::new()));
        assert!(parse_var("nope").is_err() && parse_var("=x").is_err());
    }

    #[test]
    fn without_a_terminal_only_vars_answer() {
        let host = CliHost { vars: Env::from([("code".into(), "1".into())]), interactive: false };
        let field = |name: &str| PromptVar { name: name.into(), ..PromptVar::default() };
        let ask = Ask::Fields { title: "T".into(), fields: vec![field("code"), field("otp")] };
        assert_eq!(host.answer(&ask).unwrap(), Answer::Given(Env::from([("code".into(), "1".into())])));
        let pause = Ask::Continue { title: "T".into(), text: "hi".into() };
        assert_eq!(host.answer(&pause).unwrap(), Answer::Unattended);

        let choice = |label: &str, code: &str| cactus_core::tools::Choice {
            label: label.into(),
            values: [("code".to_string(), code.to_string())].into(),
        };
        let pick = Ask::Choose { title: "T".into(), choices: vec![choice("A", "0"), choice("B", "1")] };
        assert_eq!(host.answer(&pick).unwrap(), Answer::Given(Env::from([(CHOICE.into(), "1".into())])));
    }
}

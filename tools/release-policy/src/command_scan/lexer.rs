//! Quote-aware bounded tokenization for command text.

pub(super) fn logical_lines(text: &str) -> Vec<String> {
    let joined = text.replace("\\\r\n", " ").replace("\\\n", " ");
    let mut lines = joined.lines().peekable();
    let mut logical = Vec::new();
    while let Some(line) = lines.next() {
        if line
            .trim_start()
            .strip_prefix("run:")
            .is_some_and(|value| value.trim_start().starts_with('>'))
        {
            let mut folded = "run:".to_owned();
            while let Some(next) = lines.peek() {
                if next.is_empty() || next.starts_with([' ', '\t']) {
                    folded.push(' ');
                    folded.push_str(next.trim());
                    lines.next();
                } else {
                    break;
                }
            }
            logical.push(folded);
        } else {
            logical.push(line.to_owned());
        }
    }
    logical
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum Quote {
    None,
    Single,
    Double,
}

pub(super) fn commands(line: &str) -> Result<(Vec<Vec<String>>, bool), &'static str> {
    let mut commands = Vec::new();
    let mut command = Vec::new();
    let mut token = String::new();
    let mut word_started = false;
    let mut backtick = false;
    let mut quote = Quote::None;
    let mut escaped = false;
    for character in line.chars() {
        if escaped {
            token.push(character);
            word_started = true;
            escaped = false;
            continue;
        }
        match quote {
            Quote::Single if character == '\'' => quote = Quote::None,
            Quote::Single => token.push(character),
            Quote::Double if character == '"' => quote = Quote::None,
            Quote::Double if character == '\\' => escaped = true,
            Quote::Double => {
                backtick |= character == '`';
                token.push(character);
            }
            Quote::None if character == '\'' => {
                word_started = true;
                quote = Quote::Single;
            }
            Quote::None if character == '"' => {
                word_started = true;
                quote = Quote::Double;
            }
            Quote::None if character == '\\' => {
                word_started = true;
                escaped = true;
            }
            Quote::None if character == '#' && !word_started => break,
            Quote::None if command_separator(character) => {
                push_token(&mut command, &mut token, &mut word_started);
                push_command(&mut commands, &mut command);
            }
            Quote::None if token_separator(character) => {
                push_token(&mut command, &mut token, &mut word_started);
            }
            Quote::None => {
                backtick |= character == '`';
                token.push(character);
                word_started = true;
            }
        }
    }
    if escaped {
        return Err("trailing-escape");
    }
    if quote != Quote::None {
        return Err("unterminated-quote");
    }
    push_token(&mut command, &mut token, &mut word_started);
    push_command(&mut commands, &mut command);
    Ok((commands, backtick))
}

fn command_separator(character: char) -> bool {
    matches!(character, ';' | '|' | '&' | '(' | ')' | '[' | ']')
}

fn token_separator(character: char) -> bool {
    character.is_whitespace() || matches!(character, ',' | ':')
}

fn push_token(command: &mut Vec<String>, token: &mut String, word_started: &mut bool) {
    let normalized = normalize_token(token);
    if !normalized.is_empty() {
        command.push(normalized);
    }
    token.clear();
    *word_started = false;
}

fn push_command(commands: &mut Vec<Vec<String>>, command: &mut Vec<String>) {
    if !command.is_empty() {
        commands.push(std::mem::take(command));
    }
}

fn normalize_token(token: &str) -> String {
    token
        .trim_matches('`')
        .trim_matches(['{', '}'])
        .trim_end_matches(['.', '!', '?'])
        .trim_matches('`')
        .to_owned()
}

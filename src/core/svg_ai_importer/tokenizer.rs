pub(crate) fn is_svg_path_command(character: char) -> bool {
    matches!(
        character,
        'M' | 'm'
            | 'L'
            | 'l'
            | 'H'
            | 'h'
            | 'V'
            | 'v'
            | 'C'
            | 'c'
            | 'S'
            | 's'
            | 'Q'
            | 'q'
            | 'T'
            | 't'
            | 'A'
            | 'a'
            | 'Z'
            | 'z'
    )
}

pub(crate) fn is_svg_command_token(token: &str) -> bool {
    token.len() == 1 && token.chars().next().is_some_and(is_svg_path_command)
}

pub(crate) fn tokenize_svg_path(source: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut number = String::new();
    let flush_number = |tokens: &mut Vec<String>, number: &mut String| {
        if !number.is_empty() {
            tokens.push(std::mem::take(number));
        }
    };
    for character in source.chars() {
        if is_svg_path_command(character) {
            flush_number(&mut tokens, &mut number);
            tokens.push(character.to_string());
        } else if character.is_ascii_whitespace() || character == ',' {
            flush_number(&mut tokens, &mut number);
        } else if ((matches!(character, '+' | '-') && !number.is_empty())
            || (character == '.' && number.contains('.')))
            && !number.ends_with('e')
            && !number.ends_with('E')
        {
            // SVG permits `10-20` and `1.2.3` without a comma or whitespace.
            flush_number(&mut tokens, &mut number);
            number.push(character);
        } else {
            number.push(character);
        }
    }
    flush_number(&mut tokens, &mut number);
    tokens
}

pub(crate) fn svg_command_arity(command: char) -> Option<usize> {
    match command {
        'M' | 'm' | 'L' | 'l' | 'T' | 't' => Some(2),
        'H' | 'h' | 'V' | 'v' => Some(1),
        'C' | 'c' => Some(6),
        'S' | 's' | 'Q' | 'q' => Some(4),
        'A' | 'a' => Some(7),
        'Z' | 'z' => Some(0),
        _ => None,
    }
}

pub(crate) fn next_svg_parameter_group(
    tokens: &[String],
    index: usize,
    command: char,
    arity: usize,
) -> Option<(Vec<String>, usize)> {
    if matches!(command, 'A' | 'a')
        && index + 6 <= tokens.len()
        && tokens[index + 3].len() == 2
        && tokens[index + 3]
            .chars()
            .all(|character| matches!(character, '0' | '1'))
    {
        let mut group = tokens[index..index + 3].to_vec();
        let mut flags = tokens[index + 3].chars();
        group.push(flags.next()?.to_string());
        group.push(flags.next()?.to_string());
        group.extend(tokens[index + 4..index + 6].iter().cloned());
        return Some((group, 6));
    }
    if index + arity > tokens.len()
        || tokens[index..index + arity]
            .iter()
            .any(|token| is_svg_command_token(token))
    {
        return None;
    }
    Some((tokens[index..index + arity].to_vec(), arity))
}

pub(crate) fn expand_repeated_svg_commands(tokens: Vec<String>) -> Vec<String> {
    let mut expanded = Vec::with_capacity(tokens.len());
    let mut index = 0;
    while index < tokens.len() {
        let Some(command) = tokens[index]
            .chars()
            .next()
            .filter(|_| tokens[index].len() == 1)
        else {
            index += 1;
            continue;
        };
        if !is_svg_path_command(command) {
            index += 1;
            continue;
        }
        index += 1;
        let Some(arity) = svg_command_arity(command) else {
            while index < tokens.len() && !is_svg_command_token(&tokens[index]) {
                index += 1;
            }
            continue;
        };
        expanded.push(command.to_string());
        if arity == 0 {
            continue;
        }

        let mut first_group = true;
        while index < tokens.len() && !is_svg_command_token(&tokens[index]) {
            let Some((group, consumed)) = next_svg_parameter_group(&tokens, index, command, arity)
            else {
                break;
            };
            if !first_group {
                let repeated_command = if matches!(command, 'M' | 'm') {
                    if command == 'M' {
                        'L'
                    } else {
                        'l'
                    }
                } else {
                    command
                };
                expanded.push(repeated_command.to_string());
            }
            expanded.extend(group);
            index += consumed;
            first_group = false;
        }
    }
    expanded
}

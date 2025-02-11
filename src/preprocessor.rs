//! Pre-process parser for VDF, basically making the Serde parser able to read it better.
//!  Normally, making a parser, this wouldn't be a great idea. Though, VDF has a very loose format.
//!  Essentially, this function alone, will organize the VDF into a more parseable format.
//!  For example, with a VDF that has un-ordered sequences in a map; will look like:
//!  ```vdf
//!  "test"
//!  {
//!      "name"      "Better VDF"
//!      "list"      "a"
//!      "int"       "1"
//!      "list"      "b"
//!      "list_struct"
//!      {
//!          "name"      "a"
//!          "int"       "1"
//!      }
//!      "map_to_list"
//!      {
//!          "list"      "a"
//!          "struct"
//!          {
//!              "name"       "a"
//!              "int"       "1"
//!          }
//!          "bool"      "1"
//!          "struct"
//!          {
//!              "name"       "b"
//!              "int"       "1"
//!          }
//!          "list"      "b"
//!      }
//!      "list_struct"
//!      {
//!          "name"      "b"
//!          "int"       "1"
//!      }
//!  }
//! ```
//! This function will convert it to:
//!  ```vdf
//!  "test"
//!  {
//!      "name"      "Better VDF"
//!      "list"      "a"
//!      "list"      "b"
//!      "int"       "42"
//!      "map_to_list"
//!      {
//!          "bool"      "1"
//!          "list"      "a"
//!          "list"      "b"
//!          "struct"
//!          {
//!              "name"      "a"
//!              "int"       "1"
//!          }
//!          "struct"
//!          {
//!              "name"      "b"
//!              "int"       "1"
//!          }
//!      }
//!      "list_struct"
//!      {
//!          "name"      "a"
//!          "int"       "1"
//!      }
//!      "list_struct"
//!      {
//!          "name"      "b"
//!          "int"       "1"
//!      }
//!  }
//! ```
use std::collections::HashMap;

use crate::{Error, Result};

struct ProcessingBlock {
    content: String,
    block_indent: usize,
}

impl ProcessingBlock {
    fn new(content: String, block_indent: usize) -> Self {
        ProcessingBlock {
            content,
            block_indent,
        }
    }

    fn process(&mut self) -> Result<String> {
        // println!(
        //     "{}Processing Block:\n{}",
        //     "\t".repeat(self.block_indent),
        //     self.content
        // );

        let mut mapped_content: HashMap<String, Vec<String>> = HashMap::new();
        let mut block = 0;
        let mut key: Option<String> = None;
        let mut input = self.content.as_str();
        let mut beginning = true;

        while block > 0 || beginning {
            let token = peek_real_char(input, 0)?;

            // Handle the first bracket.
            {
                if beginning && token.char != '{' {
                    return Err(Error::ExpectedMap);
                }
                if beginning {
                    block += 1;
                    // println!("Block Begining: {:?}", self.block_indent);
                    input = &input[token.index + token.char.len_utf8()..];
                    // println!("Block Begining Input: {:?}", input);
                    beginning = false;
                    continue;
                }
            }

            match token.char {
                '"' => {
                    if key.is_none() {
                        let value = parse_string(input)?;
                        key = Some(value.0);
                        // println!(
                        //     "{}Key Set: {:?}",
                        //     "\t".repeat(self.block_indent + block),
                        //     key
                        // );
                        input = value.1;
                    } else {
                        let entry_key = key.as_ref().unwrap();
                        match peek_real_char(input, 0)?.char {
                            '"' => {
                                let value = parse_string(input)?;
                                let entry = mapped_content
                                    .entry(entry_key.to_string())
                                    .or_insert_with(Vec::new);
                                entry.push(value.0.clone());

                                // println!(
                                //     "{}Entry Set[value]: {:?} - {:?}",
                                //     "\t".repeat(self.block_indent + block),
                                //     entry_key,
                                //     value.0
                                // );
                                input = value.1;
                            }
                            '{' => {
                                // This will process the block in the next iteration.
                                // println!(
                                //     "{}Prediction - Block entered: {:?}",
                                //     "\t".repeat(self.block_indent + block),
                                //     block + 1
                                // );
                                continue;
                            }
                            _ => Err(Error::ExpectedStringOrBlock)?,
                        }
                        key = None;
                    }
                }
                '{' => {
                    block += 1;
                    // println!(
                    //     "{}Block entered: {:?}",
                    //     "\t".repeat(self.block_indent + block),
                    //     block
                    // );
                    // Check, if it needs to process a step further.
                    if key.is_some() {
                        // Process the leaf input, where the block will not copy other outside blocks.
                        let leaf_input = parse_block(input)?;

                        // Process the leaf.
                        let mut leaf: ProcessingBlock =
                            ProcessingBlock::new(leaf_input.0, self.block_indent + 1);

                        let leaf_output = leaf.process()?;

                        let entry = mapped_content
                            .entry(key.as_ref().unwrap().to_string())
                            .or_insert_with(Vec::new);
                        entry.push(leaf_output.clone());

                        // Update the input from the previous leaf's actions.
                        input = leaf_input.1;

                        // The leaf process takes the ending block away.
                        block -= 1;
                        key = None;
                    } else {
                        println!(
                            "{}Block missing key: {:?}",
                            "\t".repeat(self.block_indent + block),
                            block
                        );
                        return Err(Error::ExpectedMap);
                    }
                }
                '}' => {
                    // println!(
                    //     "{}Block exited: {:?}",
                    //     "\t".repeat(self.block_indent + block),
                    //     block
                    // );
                    block -= 1;
                    key = None;
                    // After the block is processed, we need to move the input to the next character
                    input = &input[token.index + token.char.len_utf8()..];
                }
                _ => {
                    println!(
                        "{}Block[{:?}]: {:?}",
                        "\t".repeat(self.block_indent + block),
                        block,
                        token.char
                    );
                    Err(Error::ExpectedStringOrBlock)?
                }
            }
        }

        input = input.trim();

        if input.len() > 0 {
            return Err(Error::TrailingCharacters);
        }

        let mut output: String = String::new();
        // println!(
        //     "{}Mapped Content: {:?}",
        //     "\t".repeat(self.block_indent + block),
        //     mapped_content
        // );
        output += "\t".repeat(self.block_indent).as_str();
        output += "{";
        // Sort mapped_content keys.
        let mut keys: Vec<&String> = mapped_content.keys().collect();
        keys.sort();

        for key in keys {
            let value = mapped_content.get(key).unwrap();
            if value.len() > 1 {
                for v in value.iter() {
                    output += "\t".repeat(self.block_indent + 1).as_str();
                    output += format!("{}\t\"{}\"", if output.len() == 0 { "" } else { "\n" }, key)
                        .as_str();

                    if peek_expect_char(&v, 0, '{')? {
                        output += "\n";
                        let lines = v.split('\n');
                        for line in lines.clone().enumerate() {
                            output += "\t".repeat(self.block_indent + 1).as_str();
                            output += format!("{}", line.1).as_str();
                            if line.0 < lines.clone().count() - 1 {
                                output += "\n";
                            }
                        }
                    } else {
                        output += format!("\t\t\"{}\"", v).as_str();
                    }
                }
            } else {
                output += "\t".repeat(self.block_indent + 1).as_str();
                output +=
                    format!("{}\t\"{}\"", if output.len() == 0 { "" } else { "\n" }, key).as_str();
                let v = value.get(0).unwrap();

                if peek_expect_char(&v, 0, '{')? {
                    output += "\n";
                    let lines = v.split('\n');
                    for line in lines.clone().enumerate() {
                        output += "\t".repeat(self.block_indent).as_str();
                        output += format!("{}", line.1).as_str();
                        if line.0 < lines.clone().count() - 1 {
                            output += "\n";
                        }
                    }
                } else {
                    output += format!("\t\t\"{}\"", v).as_str();
                }
            }
        }
        output += "\n}";

        // println!(
        //     "{}Block Output:\n{}",
        //     "\t".repeat(self.block_indent + block),
        //     output
        // );

        Ok(output)
    }
}

pub fn preprocess(
    vdf_contents: &str,
    has_header: bool,
    keep_header: bool,
) -> crate::Result<String> {
    // println!("Preprocessing VDF:\n{:?}", vdf_contents);

    let mut input = vdf_contents;

    let mut output: String = String::new();
    if has_header {
        let header = parse_string(vdf_contents)?;
        // println!("Header Output: {:?}", header.0);
        input = header.1;
        if keep_header {
            output += format!("\"{}\"\n", header.0).as_str();
        }
    }

    let ch = peek_real_char(input, 0)?;
    match ch.char {
        '{' => {
            let mut block: ProcessingBlock = ProcessingBlock::new(input.to_string(), 0);

            let block_output = block.process()?;

            output += block_output.as_str();

            return Ok(output);
        }
        '"' => {
            if has_header {
                return Err(Error::ExpectedMap);
            }

            // println!("Found String VDF Pre-Processor...");
            let value = parse_string(input)?;
            // println!("String Output:\n{}", value.0);
            return Ok(value.0);
        }
        _ => Err(Error::ExpectedStringOrBlock),
    }
}

pub fn peek_real_char(input: &str, pointer: usize) -> Result<TokenCharacter> {
    let mut output: char = input.chars().next().ok_or(Error::Eof)?;
    let mut temp_input: &str = &input[pointer + output.len_utf8()..];
    let mut pointer = pointer;
    while output.is_whitespace() {
        let token = temp_input.chars().next().ok_or(Error::Eof)?;
        temp_input = &temp_input[token.len_utf8()..];
        pointer += token.len_utf8();
        output = token;
    }
    // println!("Peeked Char: {:?}", output);
    Ok(TokenCharacter {
        index: pointer,
        char: output,
    })
}

pub fn parse_string(input: &str) -> Result<(String, &str)> {
    let ch = peek_real_char(input, 0)?;
    if ch.char != '"' {
        println!(
            "simple_parser::parse_string failed - {:?}, because expected: '\"' retrieved: {:?}\n{:?}",
            Error::ExpectedString,
            ch.char,
            input.chars()
        );
        return Err(Error::ExpectedString);
    }
    let input = &input[ch.index + ch.char.len_utf8()..];
    let len = input.find('"');

    match len {
        Some(len) => {
            let output = &input[..len];
            // println!("Parsed String: {:?}\n{:?}", output, input);
            let input = &input[len + 1..];

            Ok((output.to_string(), &input))
        }
        None => Err(Error::Eof),
    }
}

fn parse_block(input: &str) -> Result<(String, &str)> {
    // println!("Parsing Block:\n{}", input);
    let ch = peek_real_char(input, 0)?;
    if ch.char != '{' {
        println!(
            "parse_block failed - {:?}, because expected: '{{' retrieved: {:?}\n{:?}",
            Error::ExpectedMap,
            ch.char,
            input.chars()
        );
        return Err(Error::ExpectedMap);
    }

    let mut input = &input[ch.index + ch.char.len_utf8()..];
    // println!("Block Input:\n{}", input);
    let mut output: String = "{".to_string();

    let mut block = 1;
    // println!(
    //     "Block Start: {:?} - {:?} {:?}",
    //     block,
    //     block > 0,
    //     output.len() == 0
    // );
    // println!("Block Start:\n{}", input);
    while block > 0 {
        let char = input.chars().next().ok_or(Error::Eof)?;
        input = &input[char.len_utf8()..];
        match char {
            '{' => {
                block += 1;
                // println!("Block entered: {:?}", block);
            }
            '}' => {
                block -= 1;
                // println!("Block exited: {:?}", block);
            }
            _ => {
                // println!("Block[{:?}]: {:?}", block, char);
            }
        }
        output += char.to_string().as_str();
    }
    // println!("Block End Result:\n{}", output);

    Ok((output, input))
}

pub fn peek_expect_char(input: &str, pointer: usize, expect: char) -> Result<bool> {
    let ch = peek_real_char(input, pointer)?;
    if ch.char != expect {
        return Ok(false);
    }
    Ok(true)
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct TokenCharacter {
    pub index: usize,
    pub char: char,
}

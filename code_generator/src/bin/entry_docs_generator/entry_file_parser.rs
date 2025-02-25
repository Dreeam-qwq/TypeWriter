use crate::treesitter::{print_tree, query, QueryResult};
use serde::Serialize;
use std::{fmt::Display, str::FromStr};
use thiserror::Error;

#[derive(Debug, Serialize, Clone)]
pub struct EntryClass {
    pub name: String,
    pub description: String,
    pub class_name: String,
    pub comment: Option<String>,
    pub entry_types: Vec<String>,
    pub fields: Vec<EntryField>,
}

impl EntryClass {
    pub fn is_fact(&self) -> bool {
        return self
            .entry_types
            .iter()
            .any(|entry_type| entry_type.ends_with("FactEntry"));
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct EntryField {
    pub name: String,
    pub help: Option<String>,
    pub comment: Option<String>,
    pub field_type: String,
    pub annotations: Vec<FieldAnnotation>,
}

impl EntryField {
    pub fn should_ignore_field(&self) -> bool {
        return self.name == "id" || self.name == "name";
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct FieldAnnotation {
    pub name: String,
    pub arguments: Vec<String>,
}

impl Display for FieldAnnotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.arguments.len() > 0 {
            write!(f, "@{}({})", self.name, self.arguments.join(", "))
        } else {
            write!(f, "@{}", self.name)
        }
    }
}

pub fn parse_entry_class(code: &str) -> Result<Vec<EntryClass>, EntryParseError> {
    let classes_code: QueryResult = query(&code, "(source_file (class_declaration) @class)")?;

    let classes_result: Vec<Result<EntryClass, EntryParseError>> = classes_code
        .captures
        .iter()
        .map(|cap| cap.code.parse())
        .collect();

    let mut classes: Vec<EntryClass> = Vec::new();

    for class in classes_result {
        match class {
            Ok(class) => classes.push(class),
            Err(EntryParseError::NotEntryClass) => (),
            Err(err) => return Err(err),
        }
    }

    Ok(classes)
}

impl FromStr for EntryClass {
    type Err = EntryParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let annotation = get_annotation(s)?;
        let (name, description) = parse_annotation(&annotation)?;

        return Ok(EntryClass {
            name,
            description,
            class_name: parse_class_name(s)?,
            comment: parse_comment(s).ok(),
            entry_types: parse_entry_types(s)?,
            fields: parse_entry_field(s)?,
        });
    }
}

fn get_annotation(code: &str) -> Result<String, EntryParseError> {
    let annotation_code: QueryResult = query(&code, "(class_declaration (modifiers) @annotation)")?;

    if annotation_code.captures.len() != 1 {
        return Err(EntryParseError::NotEntryClass);
    }

    let annotation = annotation_code.captures[0].code.clone();

    // Make sure this is an @Entry annotation
    if annotation.starts_with("@Entry") == false {
        return Err(EntryParseError::NotEntryClass);
    }

    Ok(annotation)
}

fn parse_annotation(code: &str) -> Result<(String, String), EntryParseError> {
    let annotation_code: QueryResult = query(&code, "(line_string_literal) @string")?;

    if annotation_code.captures.len() < 2 {
        return Err(EntryParseError::NotEntryClass);
    }

    let name = annotation_code.captures[0]
        .code
        .clone()
        .trim_start_matches('"')
        .trim_end_matches('"')
        .to_string();
    let description = annotation_code.captures[1]
        .code
        .clone()
        .trim_start_matches('"')
        .trim_end_matches('"')
        .to_string();

    Ok((name, description))
}

fn parse_class_name(code: &str) -> Result<String, EntryParseError> {
    let name_code: QueryResult = query(&code, "(class_declaration (type_identifier) @name)")?;

    if name_code.captures.len() != 1 {
        return Err(EntryParseError::FieldNotFound("name".to_string()));
    }

    let name = name_code.captures[0].code.clone();

    Ok(name)
}

fn parse_comment(code: &str) -> Result<String, EntryParseError> {
    let comment_code: QueryResult = query(&code, "(class_declaration (comment) @comment)")?;

    if comment_code.captures.len() != 1 {
        return Err(EntryParseError::FieldNotFound("comment".to_string()));
    }

    let raw_comment = comment_code.captures[0].code.clone();

    // Remove the comment characters (/**, */, *)
    let comment: String = raw_comment
        .lines()
        .map(|line| line.trim())
        .map(|line| line.trim_start_matches("/*"))
        .map(|line| line.trim_end_matches("*/"))
        .map(|line| line.trim_start_matches("*"))
        .map(|line| line.trim())
        .collect::<Vec<&str>>()
        .join("\n")
        .trim_start_matches("\n")
        .trim_end_matches("\n")
        .to_string();

    Ok(comment)
}

fn parse_entry_types(code: &str) -> Result<Vec<String>, EntryParseError> {
    let entry_type_code: QueryResult = query(
        &code,
        "(class_declaration (delegation_specifier) @class_type)",
    )?;

    let entry_types: Vec<String> = entry_type_code
        .captures
        .iter()
        .map(|cap| cap.code.clone())
        .collect();

    Ok(entry_types)
}

fn parse_entry_field(code: &str) -> Result<Vec<EntryField>, EntryParseError> {
    let fields_code: QueryResult = query(
        &code,
        "(class_declaration (primary_constructor (class_parameter) @parameter))",
    )?;

    let fields_result: Vec<Result<EntryField, EntryParseError>> = fields_code
        .captures
        .iter()
        .map(|cap| cap.code.parse())
        .collect();

    let mut fields: Vec<EntryField> = Vec::new();

    for field in fields_result {
        match field {
            Ok(field) => fields.push(field),
            Err(EntryParseError::NotEntryClass) => (),
            Err(err) => return Err(err),
        }
    }

    Ok(fields)
}

impl FromStr for EntryField {
    type Err = EntryParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let annotations = parse_field_annotations(s)?;

        return Ok(EntryField {
            name: parse_field_name(s)?,
            comment: parse_field_comment(s).ok(),
            field_type: parse_field_type(s)?,
            help: parse_field_help(&annotations),
            annotations,
        });
    }
}

fn parse_field_name(code: &str) -> Result<String, EntryParseError> {
    let name_code: QueryResult = query(
        &code,
        "(property_declaration (variable_declaration (simple_identifier) @name))",
    )?;

    if name_code.captures.len() != 1 {
        return Err(EntryParseError::FieldNotFound("name".to_string()));
    }

    let name = name_code.captures[0].code.clone();

    Ok(name)
}

fn parse_field_type(code: &str) -> Result<String, EntryParseError> {
    let field_type_code: QueryResult = query(
        &code,
        "(property_declaration (variable_declaration (user_type) @type))",
    )?;

    if field_type_code.captures.len() != 1 {
        return Err(EntryParseError::FieldNotFound("type".to_string()));
    }

    let field_type = field_type_code.captures[0].code.clone();

    Ok(field_type)
}

fn parse_field_comment(code: &str) -> Result<String, EntryParseError> {
    let comment_code: QueryResult = query(&code, "(comment) @comment")?;

    if comment_code.captures.len() != 1 {
        return Err(EntryParseError::FieldNotFound("comment".to_string()));
    }

    let raw_comment = comment_code.captures[0].code.clone();

    // Remove the comment characters (/**, */, *, //)
    let comment: String = raw_comment
        .lines()
        .map(|line| line.trim())
        .map(|line| line.trim_start_matches("/*"))
        .map(|line| line.trim_end_matches("*/"))
        .map(|line| line.trim_start_matches("*"))
        .map(|line| line.trim_start_matches("//"))
        .map(|line| line.trim())
        .collect::<Vec<&str>>()
        .join("\n")
        .trim_start_matches("\n")
        .trim_end_matches("\n")
        .to_string();

    Ok(comment)
}

fn parse_field_annotations(code: &str) -> Result<Vec<FieldAnnotation>, EntryParseError> {
    let annotation_code = query(&code, "(modifiers (annotation) @annotation)")?;

    let captures = if annotation_code.captures.len() == 0 {
        let annotation_code = query(&code, "(prefix_expression) @annotation")?;

        if annotation_code.captures.len() == 0 {
            return Ok(Vec::new());
        }

        annotation_code.captures
    } else {
        annotation_code.captures
    };

    let annotations: Result<Vec<FieldAnnotation>, EntryParseError> =
        captures.iter().map(|cap| cap.code.parse()).collect();

    annotations
}

fn parse_field_help(annotations: &Vec<FieldAnnotation>) -> Option<String> {
    let help_annotation = annotations
        .iter()
        .find(|annotation| annotation.name == "Help")?;

    let help = help_annotation.arguments.get(0)?;

    Some(help.clone())
}

impl FromStr for FieldAnnotation {
    type Err = EntryParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return Ok(FieldAnnotation {
            name: parse_annotation_name(s)?,
            arguments: parse_annotation_arguments(s)?,
        });
    }
}

fn find_annotation_from_lines(code: &str) -> Option<String> {
    // get the lines until the second @
    let mut lines = vec![];

    let mut seen_at = 0;

    for line in code.lines() {
        if line.contains("@") {
            seen_at += 1;
        }

        if seen_at == 2 {
            break;
        }

        lines.push(line);
    }

    let annotation_line = lines.join("\n");

    Some(annotation_line)
}

fn parse_annotation_name(code: &str) -> Result<String, EntryParseError> {
    let annotation_lines = find_annotation_from_lines(code).ok_or_else(|| {
        EntryParseError::AnnotationParse(format!("Could not create annotation line: {}", code))
    })?;
    // Parses if it has 0/1 arguments
    let name_code: QueryResult = query(&annotation_lines, "(annotation (user_type) @name)")?;

    let captures = if name_code.captures.len() != 1 {
        // Parses if it has 2+ arguments
        let name_code: QueryResult = query(
            &annotation_lines,
            "(annotation (constructor_invocation (user_type) @name))",
        )?;

        if name_code.captures.len() != 1 {
            return Err(EntryParseError::AnnotationParse(format!(
                "Could not parse annotation name: {}",
                code
            )));
        }

        name_code.captures
    } else {
        name_code.captures
    };

    let name = captures[0].code.clone();

    Ok(name)
}

fn parse_annotation_arguments(code: &str) -> Result<Vec<String>, EntryParseError> {
    let annotation_lines = find_annotation_from_lines(code).ok_or_else(|| {
        EntryParseError::AnnotationParse(format!("Could not create annotation line: {}", code))
    })?;
    let Ok(arguments_code) = query(
            &annotation_lines,
            "(value_argument) @argument",
        ) else {
            return Ok(Vec::new());
        };

    if arguments_code.captures.len() != 1 {
        return Ok(Vec::new());
    }

    let arguments: Vec<String> = arguments_code
        .captures
        .iter()
        .map(|cap| cap.code.clone())
        // Remove ""
        .map(|code| {
            code.trim_start_matches("\"")
                .trim_end_matches("\"")
                .trim_start_matches("(\"")
                .trim_end_matches("\")")
                .to_string()
        })
        .collect();

    Ok(arguments)
}

#[derive(Debug, Error)]
pub enum EntryParseError {
    #[error("Error parsing entry class")]
    ParseError,

    #[error("Could not find field `{0}`")]
    FieldNotFound(String),

    #[error("Could parse annotation `{0}`")]
    AnnotationParse(String),

    #[error("Not a entry class")]
    NotEntryClass,

    #[error("Error parsing query")]
    QueryError(#[from] crate::treesitter::QueryError),
}

#![allow(dead_code, unused_variables, unused_imports)]
use std::ops::Range;

#[derive(Default, Clone, Debug)]
pub struct ArgumentsState {
    pub arguments: Vec<Argument>,
    pub invalid_arguments_char_ranges: Vec<Range<usize>>,
}

#[derive(Default, Clone, Debug)]
pub struct Argument {
    pub name: String,
    pub value: String,
    pub description: String,
    pub default_value: String,
}

impl ArgumentsState {
    pub fn for_command_workflow(_prev: &Self, _content: String) -> Self {
        Self::default()
    }

    pub fn merged_arguments(&self) -> Vec<Argument> {
        self.arguments.clone()
    }
}
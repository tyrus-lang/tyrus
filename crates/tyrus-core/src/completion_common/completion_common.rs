#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsoleMember {
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrintFunction {
    pub name: &'static str,
    pub description: &'static str,
}

pub const CONSOLE_MEMBERS: &[ConsoleMember] = &[
    ConsoleMember {
        name: "log",
        description: "write to stdout",
    },
    ConsoleMember {
        name: "error",
        description: "write to stderr",
    },
    ConsoleMember {
        name: "warn",
        description: "write a warning to stderr",
    },
];

pub const PRINT_FUNCTIONS: &[PrintFunction] = &[
    PrintFunction {
        name: "print",
        description: "write a line to stdout",
    },
    PrintFunction {
        name: "printError",
        description: "write an error line to stderr",
    },
    PrintFunction {
        name: "printWarn",
        description: "write a warning line to stderr",
    },
    PrintFunction {
        name: "printRaw",
        description: "write to stdout without a newline",
    },
    PrintFunction {
        name: "printCR",
        description: "update the current stdout line",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn console_members_are_documented_and_ordered() {
        let names: Vec<&str> = CONSOLE_MEMBERS.iter().map(|member| member.name).collect();
        assert_eq!(names, vec!["log", "error", "warn"]);
        for member in CONSOLE_MEMBERS {
            assert!(
                !member.description.is_empty(),
                "{} lacks a description",
                member.name
            );
        }
    }

    #[test]
    fn print_functions_are_documented_and_ordered() {
        let names: Vec<&str> = PRINT_FUNCTIONS
            .iter()
            .map(|function| function.name)
            .collect();
        assert_eq!(
            names,
            vec!["print", "printError", "printWarn", "printRaw", "printCR"]
        );
        for function in PRINT_FUNCTIONS {
            assert!(
                !function.description.is_empty(),
                "{} lacks a description",
                function.name
            );
        }
    }
}

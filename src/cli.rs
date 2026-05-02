use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Args {
    pub repo_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    TooManyArguments,
}

impl Args {
    pub fn parse_from<I, S>(args: I) -> Result<Self, ParseError>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut args = args.into_iter().map(Into::into);
        let _program = args.next();
        let repo_path = args.next().map(PathBuf::from);

        if args.next().is_some() {
            return Err(ParseError::TooManyArguments);
        }

        Ok(Self { repo_path })
    }
}

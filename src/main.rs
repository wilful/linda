use chrono::prelude::*;
use std::str::FromStr;
use std::{fmt, env};
use rusqlite::{Connection, Result};
use error::*;

mod cli {
    use super::*;
    use clap::{Parser, Subcommand};

    #[derive(Parser, Debug)]
    #[command(name = "linda")]
    #[command(author, version, about, long_about = None)]
    struct Cli {
        #[command(subcommand)]
        command: Commands,
    }

    #[derive(Debug, Subcommand)]
    enum Commands {
        #[command(arg_required_else_help = true)]
        Exec {
            #[arg(short, long, default_value_t = String::from("&100,10,some word,other word"))]
            text: String,
        },
        Init {}
    }

    pub fn call() {
        let args = Cli::parse();
        match args.command {
            Commands::Exec { text } => {
                let cmd = match Cmd::from_str(&text) {
                    Ok(c) => c,
                    Err(e) => panic!("[error] {e:?}: {e}"),
                };
                run(cmd);
            },
            Commands::Init {} => {
                init().expect("Can't initializing database");
            },
        }
    }
}

mod error {
    use super::*;

    #[derive(Debug)]
    pub struct ParseCmdError;
    #[derive(Debug)]
    pub struct NoSpecifiedOrderKindError;

    impl fmt::Display for ParseCmdError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "The first character in the command line does not match the allowed characters")
        }
    }

    impl fmt::Display for NoSpecifiedOrderKindError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "There is no operation type for the specified command")
        }
    }
}

const MODS: [char; 3] = ['&', '>', '+'];
const SEP: char = ',';
const DATABASE_FILENAME: &str = "linda.db";

#[derive(Debug)]
struct Cmd {
    pack: Vec<PartOfCmdKind>,
    created_at: DateTime<Local>
}

#[derive(Debug)]
enum PartOfCmdKind {
    Mod(char),
    Digit(i32),
    Word(String),
}

trait FromKind {
    fn from_kind(k: &PartOfCmdKind) -> Self;
}

impl FromKind for i32 {
    fn from_kind(d: &PartOfCmdKind) -> i32 { d.unwrap_digit() }
}

impl FromKind for String {
    fn from_kind(w: &PartOfCmdKind) -> String { w.unwrap_word() }
}

impl PartOfCmdKind {
    fn unwrap_digit(&self) -> i32 {
        match self {
            PartOfCmdKind::Digit(d) => d.clone(),
            _ => panic!("[error]: expected Digit, got {:?}", self)
        }
    }
    fn unwrap_word(&self) -> String {
        match self {
            PartOfCmdKind::Word(w) => w.clone(),
            _ => panic!("[error]: expected Word, got {:?}", self)
        }
    }
    fn unwrap<T: FromKind>(&self) -> T { T::from_kind(self) }
}

#[derive(Debug)]
enum OrderKind {
    Income,
    Expense,
}

#[derive(Debug)]
enum CmdKind {
    Order(OrderKind),
}

impl fmt::Display for PartOfCmdKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PartOfCmdKind::Mod(ch) => write!(f, "{}", ch),
            PartOfCmdKind::Digit(d) => write!(f, "{}", d),
            PartOfCmdKind::Word(w) => write!(f, "{}", w)
        }
    }
}

impl OrderKind {
    fn new(ch: char) -> Result<OrderKind, NoSpecifiedOrderKindError> {
        match ch {
            '&' => Ok(OrderKind::Income),
            '>' => Ok(OrderKind::Expense),
            _ => Err(NoSpecifiedOrderKindError)
        }
    }
}

#[derive(Debug)]
struct Tr {
    created_at: DateTime<Local>,
    tax: i32,
    category: String
}

impl Tr {
    fn new(cmd: Cmd) -> Option<Tr> {
        match cmd.kind_of() {
            Some(CmdKind::Order(OrderKind::Income)) =>
                Some(Tr {
                    created_at: cmd.created_at,
                    tax: cmd.pack[1].unwrap(),
                    category: cmd.pack[2].unwrap(),
                }),
            _ => None
        }
    }
}

impl Cmd {
    fn to_sql(&self) -> Option<String> {
        match self.kind_of() {
            Some(CmdKind::Order(OrderKind::Income)) => format!(
                "INSERT INTO transaction (created_at, tax, category) VALUES ({}, {}, '{}')", self.created_at, self.pack[1], self.pack[2]
                ).into(),
            _ => None
        }
    }
    fn kind_of(&self) -> Option<CmdKind> {
        match self.pack[..] {
            [
                PartOfCmdKind::Mod(ch),
                PartOfCmdKind::Digit(_),
                PartOfCmdKind::Word(_),
            ] => Some(CmdKind::Order(OrderKind::new(ch).unwrap_or_else( |e| {
                panic!("[error] {e:?}: {e}");
            }))),
            _ => None,
        }
    }
}

impl FromStr for Cmd {
    type Err = ParseCmdError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let created_at = Local::now();
        let mut chars = text.chars();
        let ch = chars.next().unwrap();
        if !MODS.contains(&ch) { return Err(ParseCmdError); }
        let mut pack: Vec<PartOfCmdKind> = vec![
            PartOfCmdKind::Mod(ch)
        ];

        for mut ch in chars.as_str().split(SEP) {
            ch = ch.trim();
            if let Ok(d) = ch.parse::<i32>() {
                pack.push(PartOfCmdKind::Digit(d))
            } else {
                pack.push(PartOfCmdKind::Word(String::from(ch)))
            }
        }
        println!("Cmd {:?} created at {}", pack, created_at);
        Ok(Cmd { pack, created_at})
    }
}

fn init() -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open(DATABASE_FILENAME)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS `transaction` (
          id INTEGER PRIMARY KEY AUTOINCREMENT,
          created_at INTEGER NOT NULL,
          tax INTEGER NOT NULL,
          category TEXT NOT NULL,
          duration INTEGER DEFAULT 0,
          description TEXT
        )",
        (), // empty list of parameters.
    )?;
    Ok(())
}

fn run(cmd: Cmd) {
    println!("{:?}", cmd.to_sql().unwrap());
    let transaction = Tr::new(cmd).unwrap();
    println!("{:?}", transaction);
    println!("{:?}, {}, {}", transaction.created_at, transaction.tax, transaction.category);
    println!("{:?}", env::current_dir());
}

fn main() {
    cli::call();
}

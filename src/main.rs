// NOTES TO SELF:
//  - CHECK EACH COMMAND
//  - HANDLE WHEN SONDE IS EMPTY

use chrono::Local;
use clap::{Parser, Subcommand};
use regex::Regex;
use std::fs;
use std::path::PathBuf;

fn sonde_file() -> PathBuf {
    dirs::home_dir().unwrap().join(".sonde")
}

struct Task {
    status: char,
    text: String,
    prob: f64,
    time: f64,
    real_time: Option<f64>,
    reason: Option<String>,
    timestamp: String,
}

impl Task {
    fn to_line(&self) -> String {
        let mut s = format!(
            "[{}] {} p:{} t:{}",
            self.status, self.text, self.prob, self.time
        );
        if let Some(r) = self.real_time {
            s.push_str(&format!(" r:{r}"));
        }
        if let Some(ref reason) = self.reason {
            s.push_str(&format!(" \"{reason}\""));
        }
        s.push_str(&format!(" @{}", self.timestamp));
        s
    }
}

fn parse_line(line: &str) -> Option<Task> {
    let re = Regex::new(
        r#"\[(.)\] (.+?) p:([\d.]+) t:([\d.]+)(?: r:([\d.]+))?(?: "([^"]+)")? @(\S+)$"#,
    )
    .unwrap();
    let caps = re.captures(line)?;
    Some(Task {
        status: caps[1].chars().next()?,
        text: caps[2].to_string(),
        prob: caps[3].parse().ok()?,
        time: caps[4].parse().ok()?,
        real_time: caps.get(5).and_then(|m| m.as_str().parse().ok()),
        reason: caps.get(6).map(|m| m.as_str().to_string()),
        timestamp: caps[7].to_string(),
    })
}

fn load_tasks() -> Vec<Task> {
    let path = sonde_file();
    if !path.exists() {
        return vec![];
    }
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter_map(parse_line)
        .collect()
}

fn save_tasks(tasks: &[Task]) {
    let content: String = tasks.iter().map(|t| t.to_line() + "\n").collect();
    fs::write(sonde_file(), content).unwrap();
}

fn priority(prob: f64, time: f64) -> f64 {
    -prob.ln() / time
}

fn top_task_idx(tasks: &[Task]) -> Option<usize> {
    tasks
        .iter()
        .enumerate()
        .filter(|(_, t)| t.status == ' ')
        .max_by(|(_, a), (_, b)| {
            priority(a.prob, a.time)
                .partial_cmp(&priority(b.prob, b.time))
                .unwrap()
        })
        .map(|(i, _)| i)
}

fn show_top(tasks: &[Task]) {
    if let Some(i) = top_task_idx(tasks) {
        let t = &tasks[i];
        println!("→ [{:.2}] {}", priority(t.prob, t.time), t.text);
    }
}

#[derive(Parser)]
#[command(name = "sonde")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Add a new task
    Add {
        text: String,
        #[arg(short, long)]
        prob: f64,
        #[arg(short, long)]
        time: f64,
    },
    /// Show the highest priority task
    Show,
    /// Complete the top task
    Next {
        #[arg(short, long)]
        real_time: f64,
        reason: Option<String>,
    },
    /// Fail the top task
    Fail {
        #[arg(short, long)]
        real_time: f64,
        reason: Option<String>,
    },
    /// Abandon the top task
    Abandon { reason: Option<String> },
    /// List all open tasks by priority
    List,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Add { text, prob, time }) => {
            println!("[command] add");
            let mut tasks = load_tasks();
            tasks.push(Task {
                status: ' ',
                text,
                prob,
                time,
                real_time: None,
                reason: None,
                timestamp: Local::now().format("%Y-%m-%d").to_string(),
            });
            save_tasks(&tasks);
        }
        Some(Command::Show) | None => {
            println!("[command] show");
            let tasks = load_tasks();
            show_top(&tasks);
        }
        Some(Command::Next { real_time, reason }) => {
            println!("[command] next");
            if real_time < 0.0 {
                println!("[error] positive real_time is required for completing task");
                return;
            }
            let mut tasks = load_tasks();
            if let Some(i) = top_task_idx(&tasks) {
                tasks[i].status = 'x';
                tasks[i].real_time = Some(real_time);
                tasks[i].reason = reason;
                save_tasks(&tasks);
            }
            show_top(&tasks);
        }
        Some(Command::Fail { real_time, reason }) => {
            println!("[command] fail");
            if real_time < 0.0 {
                println!("[error] positive real_time is required for completing task");
                return;
            }
            let mut tasks = load_tasks();
            if let Some(i) = top_task_idx(&tasks) {
                tasks[i].status = 'f';
                tasks[i].real_time = Some(real_time);
                tasks[i].reason = reason;
                save_tasks(&tasks);
            }
            show_top(&tasks);
        }
        Some(Command::Abandon { reason: _ }) => {
            println!("[command] abandon");
        }
        Some(Command::List) => {
            println!("[command] list");
            let tasks = load_tasks();
            let mut open: Vec<&Task> = tasks.iter().filter(|t| t.status == ' ').collect();
            open.sort_by(|a, b| {
                priority(b.prob, b.time)
                    .partial_cmp(&priority(a.prob, a.time))
                    .unwrap()
            });
            for task in open {
                println!("→ [{:.2}] {}", priority(task.prob, task.time), task.text);
            }
        }
    }
}

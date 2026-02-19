// NOTES TO SELF:
//  - HANDLE WHEN SONDE IS EMPTY

use chrono::Local;
use clap::{Parser, Subcommand};
use regex::Regex;
use std::fs;
use std::path::PathBuf;

fn sonde_file() -> PathBuf {
    dirs::home_dir().unwrap().join(".sonde")
}

fn timer_file() -> PathBuf {
    dirs::home_dir().unwrap().join(".sonde_timer")
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

fn start_timer() {
    let now = Local::now().timestamp();
    fs::write(timer_file(), now.to_string()).unwrap();
}

fn stop_timer() -> Option<f64> {
    let path = timer_file();
    if !path.exists() {
        return None;
    }
    let start: i64 = fs::read_to_string(&path).unwrap().trim().parse().unwrap();
    let elapsed = Local::now().timestamp() - start;
    fs::remove_file(path).unwrap();
    Some(elapsed as f64 / 3600.0)
}

fn elapsed_str() -> Option<String> {
    let path = timer_file();
    if !path.exists() {
        return None;
    }
    let start: i64 = fs::read_to_string(&path).ok()?.trim().parse().ok()?;
    let secs = Local::now().timestamp() - start;
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    if hours > 0 {
        Some(format!("{hours}h {mins}m"))
    } else {
        Some(format!("{mins}m"))
    }
}

fn show_top(tasks: &[Task]) {
    if let Some(i) = top_task_idx(tasks) {
        let t = &tasks[i];
        let elapsed = elapsed_str().map(|e| format!(" ({e})")).unwrap_or_default();
        println!(
            "→ [{:.2}] {}{elapsed}",
            priority(t.prob, t.time),
            t.text
        );
    }
}

#[derive(Parser)]
#[command(name = "sonde")]
#[command(about = "Prioritize tasks by failure rate per unit time. Based on Steinhardt's 'Research as a Stochastic Decision Process'.")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Add a new task with success probability and estimated time in hours
    Add {
        /// Description of the task
        text: String,
        /// Probability of success (0.0 to 1.0). Use ~0.95 for confident, ~0.65 for plausible, ~0.30 for murky
        #[arg(short, long)]
        prob: f64,
        /// Estimated time to complete in hours
        #[arg(short, long)]
        time: f64,
    },
    /// Show the highest priority task (default when no command given)
    Show,
    /// Start the timer on the current top task
    Start,
    /// Mark the top task as completed. Uses timer if running, otherwise pass -r
    Next {
        /// Actual time spent in hours (auto-filled if timer is running)
        #[arg(short, long)]
        real_time: Option<f64>,
        /// What happened, what you learned
        reason: Option<String>,
    },
    /// Mark the top task as failed. Uses timer if running, otherwise pass -r
    Fail {
        /// Actual time spent in hours (auto-filled if timer is running)
        #[arg(short, long)]
        real_time: Option<f64>,
        /// Why it failed — what was the blocker
        reason: Option<String>,
    },
    /// Rule out the top task early without fully attempting it
    Abandon {
        /// Why you're ruling it out
        reason: Option<String>,
    },
    /// Update probability or time estimate on the current top task
    Edit {
        /// New probability of success
        #[arg(short, long)]
        prob: Option<f64>,
        /// New estimated time in hours
        #[arg(short, long)]
        time: Option<f64>,
    },
    /// List all open tasks sorted by priority (highest first)
    List,
    /// Show calibration stats: how well your estimates match reality
    Stats,
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
        Some(Command::Start) => {
            println!("[command] start");
            let tasks = load_tasks();
            if let Some(i) = top_task_idx(&tasks) {
                start_timer();
                println!("→ timing: {}", tasks[i].text);
            }
        }
        Some(Command::Next { real_time, reason }) => {
            println!("[command] next");
            let real_time = real_time.or_else(stop_timer);
            let Some(real_time) = real_time else {
                println!("[error] no timer running. pass -r <hours>");
                return;
            };
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
            let real_time = real_time.or_else(stop_timer);
            let Some(real_time) = real_time else {
                println!("[error] no timer running. pass -r <hours>");
                return;
            };
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
        Some(Command::Abandon { reason }) => {
            println!("[command] abandon");
            let mut tasks = load_tasks();
            if let Some(i) = top_task_idx(&tasks) {
                tasks[i].status = 'a';
                tasks[i].real_time = stop_timer().or(Some(0.0));
                tasks[i].reason = reason;
                save_tasks(&tasks);
            }
            show_top(&tasks);
        }
        Some(Command::Edit { prob, time }) => {
            println!("[command] edit");
            if prob.is_none() && time.is_none() {
                println!("[error] pass -p <prob> and/or -t <hours> to update");
                return;
            }
            let mut tasks = load_tasks();
            if let Some(i) = top_task_idx(&tasks) {
                if let Some(p) = prob {
                    tasks[i].prob = p;
                }
                if let Some(t) = time {
                    tasks[i].time = t;
                }
                println!(
                    "→ {} p:{} t:{}",
                    tasks[i].text, tasks[i].prob, tasks[i].time
                );
                save_tasks(&tasks);
            }
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
        Some(Command::Stats) => {
            println!("[command] stats");
            let tasks = load_tasks();
            let done: Vec<&Task> = tasks
                .iter()
                .filter(|t| t.status != ' ' && t.real_time.is_some())
                .collect();

            if done.is_empty() {
                println!("no completed tasks yet");
                return;
            }

            let total = done.len();
            let succeeded = done.iter().filter(|t| t.status == 'x').count();
            let failed = done.iter().filter(|t| t.status == 'f').count();
            let abandoned = done.iter().filter(|t| t.status == 'a').count();

            println!("{total} tasks: {succeeded} succeeded, {failed} failed, {abandoned} abandoned");

            // time calibration: average ratio of real_time / estimated time
            let ratios: Vec<f64> = done
                .iter()
                .filter(|t| t.status == 'x' || t.status == 'f')
                .filter_map(|t| t.real_time.map(|r| r / t.time))
                .collect();

            if !ratios.is_empty() {
                let avg_ratio: f64 = ratios.iter().sum::<f64>() / ratios.len() as f64;
                println!(
                    "time calibration: tasks take {:.1}x your estimate on average",
                    avg_ratio
                );
            }

            // probability calibration: did ~90% of your p:0.9 tasks actually succeed?
            let buckets = [(0.0, 0.5), (0.5, 0.75), (0.75, 0.95), (0.95, 1.01)];
            let has_bucket_data = done.iter().any(|t| t.status == 'x' || t.status == 'f');
            if has_bucket_data {
                println!("prob calibration:");
                for (lo, hi) in buckets {
                    let in_bucket: Vec<&&Task> = done
                        .iter()
                        .filter(|t| t.status == 'x' || t.status == 'f')
                        .filter(|t| t.prob >= lo && t.prob < hi)
                        .collect();
                    if in_bucket.is_empty() {
                        continue;
                    }
                    let wins = in_bucket.iter().filter(|t| t.status == 'x').count();
                    let n = in_bucket.len();
                    println!(
                        "  p:{:.0}-{:.0}% → {}/{} succeeded ({:.0}%)",
                        lo * 100.0,
                        hi.min(1.0) * 100.0,
                        wins,
                        n,
                        wins as f64 / n as f64 * 100.0
                    );
                }
            }
        }
    }
}

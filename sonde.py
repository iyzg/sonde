### NOTES TO SELF:
###  - CHECK EACH COMMAND
###  - HANDLE WHEN SONDE IS EMPTY

import argparse
from dataclasses import dataclass
import re
import sys
from pathlib import Path
from datetime import date
from math import log

from pprint import pprint
SONDE_FILE = Path.home() / ".sonde"


@dataclass
class Task:
    status: str
    text: str
    prob: float
    time: float
    real_time: float | None
    reason: str | None
    timestamp: str

    def to_line(self):
        return f'[{self.status}] {self.text} p:{self.prob} t:{self.time} {f"r:{self.real_time} " if self.real_time else ""}{f'"{self.reason}" ' if self.reason else ""}@{self.timestamp}'


def load_tasks():
    if not SONDE_FILE.exists():
        return []
    with open(SONDE_FILE, "r") as f:
        return [parse_line(line) for line in f.readlines()]


def save_tasks(tasks: list[Task]):
    with open(SONDE_FILE, "w") as f:
        for task in tasks:
            f.write(task.to_line() + "\n")


def parse_line(line):
    """Parse: [x] Read papers p:0.9 t:2.0 r:2.1 "reason" @2025-11-22"""
    pattern = (
        r'\[(.)\] (.+?) p:([\d.]+) t:([\d.]+)(?: r:([\d.]+))?(?: "([^"]+)")? @(\S+)$'
    )

    match = re.match(pattern, line)
    if match:
        status = match.group(1)
        text = match.group(2)
        prob = float(match.group(3))
        time = float(match.group(4))
        real_time = float(match.group(5)) if match.group(5) else None
        reason = match.group(6) if match.group(6) else None
        timestamp = match.group(7)

    return Task(
        status=status,
        text=text,
        prob=prob,
        time=time,
        real_time=real_time,
        reason=reason,
        timestamp=timestamp
    )


def calculate_priority(prob: float, time: float) -> float:
    # The less probable and less time, the higher priority
    return -log(prob) / time


def get_top_task(tasks: list[Task]) -> Task:
    return max(tasks, key=lambda x: calculate_priority(x.prob, x.time))


def cmd_add(args):
    print("[command] add")
    if not args.text or not args.prob or not args.time:
        print("[error] text, prob, and time are all required for adding task")
        return

    tasks = load_tasks()
    tasks.append(Task(status=" ", text=args.text, prob=args.prob, time=args.time, real_time=None, reason=None, timestamp=date.today()))
    save_tasks(tasks)


def cmd_show():
    print("[command] show")

    tasks = load_tasks()
    tasks = list(filter(lambda x: x.status == " ", tasks))
    top_task = get_top_task(tasks)
    print(f"→ [{calculate_priority(top_task.prob, top_task.time):.2f}] {top_task.text}")



def cmd_next(args):
    print("[command] next")
    real_time = args.real_time
    if real_time is None or real_time < 0:
        print("[error] positive real_time is required for completing task")
        return

    tasks = load_tasks()
    open_tasks = list(filter(lambda x: x.status == " ", tasks))
    top_task = get_top_task(open_tasks)
    top_task.status = "x"
    top_task.real_time = args.real_time
    top_task.reason = args.reason
    save_tasks(tasks)

    cmd_show()


def cmd_fail(args):
    print("[command] fail")
    real_time = args.real_time
    if real_time is None or real_time < 0:
        print("[error] positive real_time is required for completing task")
        return

    tasks = load_tasks()
    open_tasks = list(filter(lambda x: x.status == " ", tasks))
    top_task = get_top_task(open_tasks)
    top_task.status = "f"
    top_task.real_time = args.real_time
    top_task.reason = args.reason
    save_tasks(tasks)


    cmd_show()


def cmd_abandon(args):
    print("[command] abandon")
    pass


def cmd_list():
    print("[command] list")
    tasks = load_tasks()
    open_tasks = list(filter(lambda x: x.status == " ", tasks))
    open_tasks.sort(key=lambda x: calculate_priority(x.prob, x.time), reverse=True)
    for task in open_tasks:
        print(f"→ [{calculate_priority(task.prob, task.time):.2f}] {task.text}")


if __name__ == "__main__":
    # tasks = load_tasks()
    # pprint(tasks)


    parser = argparse.ArgumentParser()
    # Commands and then also when no command, just show next task
    parser.add_argument(
        "command",
        nargs="?",
        choices=["add", "show", "next", "fail", "abandon", "list"],
        help="Command to execute",
    )
    parser.add_argument("text", nargs="?", help="Text for the command", type=str)
    parser.add_argument(
        "-p", "--prob", nargs="?", help="Probability for the command", type=float
    )
    parser.add_argument(
        "-t", "--time", nargs="?", help="Time for the command", type=float
    )
    parser.add_argument(
        "-r", "--real_time", nargs="?", help="Real time for the command", type=float
    )
    parser.add_argument("reason", nargs="?", help="Reason for the command", type=str)

    args = parser.parse_args()
    if args.command:
        if args.command == "add":
            cmd_add(args)
        elif args.command == "show":
            cmd_show()
        elif args.command == "next":
            cmd_next(args)
        elif args.command == "fail":
            cmd_fail(args)
        elif args.command == "abandon":
            cmd_abandon(args)
        elif args.command == "list":
            cmd_list()
    else:
        cmd_show()

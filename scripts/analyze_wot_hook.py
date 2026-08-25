#!/usr/bin/env python3
"""Estimate the token cost and benefit of the wot Codex hook from JSONL rollouts.

The analyzer is deliberately dependency-free and streams every rollout.  It uses
the exact ``original_token_count`` recorded by tool outputs when available and a
documented UTF-8-bytes/4 estimate otherwise.  Savings are observational, not a
causal claim: the report compares equal windows around the first exact hook
message and also shows a one-wot-replaces-one-broad-read model.
"""

from __future__ import annotations

import argparse
import bisect
import dataclasses
import datetime as dt
import json
import math
import re
import shlex
import statistics
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable


DEFAULT_HOOK_TEXT = "Use wot for a file overview."
DEFAULT_MESSAGE_OVERHEAD_TOKENS = 4
CMD_JSON_RE = re.compile(r'["\'](?:cmd|command)["\']\s*:\s*("(?:\\.|[^"\\])*")')


@dataclasses.dataclass
class ToolCall:
    timestamp: dt.datetime
    session_id: str
    cwd: str
    call_id: str
    command: str | None
    category: str
    output_tokens: int = 0
    exact_output_tokens: bool = False
    output_text: str = ""


@dataclasses.dataclass
class HookEvent:
    timestamp: dt.datetime
    session_id: str
    text: str
    rollouts: set[str]


@dataclasses.dataclass
class PromptEvent:
    timestamp: dt.datetime
    session_id: str
    hooks_seen_by_text: dict[str, int]
    input_tokens: int
    cached_input_tokens: int


@dataclasses.dataclass
class Corpus:
    files: int = 0
    incomplete_files_skipped: int = 0
    unsettled_files_skipped: int = 0
    lines: int = 0
    malformed_lines: int = 0
    sessions: set[str] = dataclasses.field(default_factory=set)
    calls: list[ToolCall] = dataclasses.field(default_factory=list)
    hooks: list[HookEvent] = dataclasses.field(default_factory=list)
    prompts: list[PromptEvent] = dataclasses.field(default_factory=list)
    first_timestamp: dt.datetime | None = None
    last_timestamp: dt.datetime | None = None


def parse_timestamp(value: str) -> dt.datetime:
    return dt.datetime.fromisoformat(value.replace("Z", "+00:00"))


def estimated_tokens(text: str) -> int:
    """Return the reproducible bytes/4 approximation used for missing counts."""
    return math.ceil(len(text.encode("utf-8")) / 4)


def message_text(payload: dict[str, Any]) -> str:
    parts = []
    content = payload.get("content")
    if not isinstance(content, list):
        return ""
    for item in content:
        if isinstance(item, dict) and isinstance(item.get("text"), str):
            parts.append(item["text"])
    return "".join(parts)


def json_commands(source: str) -> list[str]:
    commands = []
    for match in CMD_JSON_RE.finditer(source):
        try:
            commands.append(json.loads(match.group(1)))
        except json.JSONDecodeError:
            continue
    return commands


def command_from_payload(payload: dict[str, Any]) -> str | None:
    for key in ("arguments", "input"):
        value = payload.get(key)
        if isinstance(value, dict):
            for command_key in ("cmd", "command"):
                if isinstance(value.get(command_key), str):
                    return value[command_key]
        elif isinstance(value, str):
            try:
                decoded = json.loads(value)
            except json.JSONDecodeError:
                decoded = None
            if isinstance(decoded, dict):
                for command_key in ("cmd", "command"):
                    if isinstance(decoded.get(command_key), str):
                        return decoded[command_key]
            commands = json_commands(value)
            if commands:
                return "\n".join(commands)

    action = payload.get("action")
    if isinstance(action, dict):
        command = action.get("command")
        if isinstance(command, str):
            return command
        if isinstance(command, list) and all(isinstance(part, str) for part in command):
            return " ".join(shlex.quote(part) for part in command)
    return None


def split_shell_segments(command: str) -> Iterable[str]:
    # This intentionally favors recall for analysis.  The exact hook classifier
    # is narrower and is not used to claim that a reminder fired.
    return (segment.strip() for segment in re.split(r"(?:&&|\|\||[;|\n])", command))


def shell_words(command: str) -> list[str]:
    try:
        return shlex.split(command)
    except ValueError:
        return command.split()


def unwrap_prefixes(words: list[str]) -> list[str]:
    while words and words[0] in {"command", "env", "sudo"}:
        words = words[1:]
        if words and words[0] == "--":
            words = words[1:]
    return words


def sed_span_is_broad(words: list[str]) -> bool:
    if not any(word == "-n" or (word.startswith("-") and "n" in word) for word in words):
        return True
    for word in words:
        match = re.fullmatch(r"(\d+),(\d+)p?", word)
        if match and int(match.group(2)) - int(match.group(1)) + 1 > 80:
            return True
    return False


def segment_is_broad(segment: str) -> bool:
    words = unwrap_prefixes(shell_words(segment))
    if not words:
        return False
    executable = Path(words[0]).name
    if executable in {"cat", "head", "tail", "nl", "find", "fd"}:
        return True
    if executable == "ls":
        return any(word.startswith("-") and "R" in word for word in words[1:])
    if executable == "sed":
        return sed_span_is_broad(words)
    if executable in {"rg", "grep", "ripgrep"}:
        args = words[1:]
        if any(word in {"--files", "-l", "--files-with-matches"} for word in args):
            return False
        return all(word.startswith("-") or word == "." for word in args)
    return False


def segment_is_wot(segment: str) -> bool:
    words = unwrap_prefixes(shell_words(segment))
    return bool(words) and Path(words[0]).name == "wot"


def classify_command(command: str | None) -> str:
    if not command:
        return "other"
    segments = list(split_shell_segments(command))
    if any(segment_is_wot(segment) for segment in segments):
        return "wot"
    if any(segment_is_broad(segment) for segment in segments):
        return "broad"
    return "other"


def command_cwd_and_segments(command: str, initial_cwd: str) -> tuple[Path, list[list[str]]]:
    current = Path(initial_cwd or ".")
    commands: list[list[str]] = []
    for segment in split_shell_segments(command):
        words = unwrap_prefixes(shell_words(segment))
        if not words:
            continue
        if Path(words[0]).name == "cd" and len(words) == 2:
            candidate = Path(words[1]).expanduser()
            current = candidate if candidate.is_absolute() else current / candidate
            continue
        commands.append(words)
    return current, commands


def single_wot_file(call: ToolCall) -> str | None:
    if call.category != "wot" or not call.command:
        return None
    cwd, segments = command_cwd_and_segments(call.command, call.cwd)
    wot_segments = [words for words in segments if Path(words[0]).name == "wot"]
    if len(wot_segments) != 1:
        return None
    words = wot_segments[0][1:]
    value_options = {"--format", "--language", "--max-depth", "--max-items", "--min-lines"}
    files: list[str] = []
    skip_next = False
    for word in words:
        if skip_next:
            skip_next = False
        elif word in value_options:
            skip_next = True
        elif word.startswith("-"):
            continue
        else:
            files.append(word)
    if len(files) != 1 or files[0] in {"setup", "hook-check"}:
        return None
    path = Path(files[0]).expanduser()
    return str((path if path.is_absolute() else cwd / path).resolve(strict=False))


def single_cat_file(call: ToolCall) -> str | None:
    if call.category != "broad" or not call.command:
        return None
    cwd, segments = command_cwd_and_segments(call.command, call.cwd)
    cats = [words for words in segments if Path(words[0]).name == "cat"]
    if len(cats) != 1:
        return None
    files = [word for word in cats[0][1:] if not word.startswith("-")]
    if len(files) != 1:
        return None
    path = Path(files[0]).expanduser()
    return str((path if path.is_absolute() else cwd / path).resolve(strict=False))


def is_tool_call_type(payload_type: str) -> bool:
    return payload_type.endswith("_call") and not payload_type.endswith("_call_output")


def output_call_id(payload: dict[str, Any]) -> str | None:
    for key in ("call_id", "id"):
        value = payload.get(key)
        if isinstance(value, str):
            return value
    return None


def original_token_counts(value: Any) -> list[int]:
    counts: list[int] = []
    if isinstance(value, dict):
        count = value.get("original_token_count")
        if isinstance(count, int):
            counts.append(count)
        for child in value.values():
            counts.extend(original_token_counts(child))
    elif isinstance(value, list):
        for child in value:
            counts.extend(original_token_counts(child))
    elif isinstance(value, str):
        try:
            decoded = json.loads(value)
        except json.JSONDecodeError:
            return counts
        counts.extend(original_token_counts(decoded))
    return counts


def visible_output_text(value: Any) -> str:
    parts: list[str] = []
    if isinstance(value, str):
        parts.append(value)
    elif isinstance(value, dict):
        if isinstance(value.get("text"), str):
            parts.append(value["text"])
        else:
            for child in value.values():
                parts.append(visible_output_text(child))
    elif isinstance(value, list):
        for child in value:
            parts.append(visible_output_text(child))
    return "".join(parts)


def output_tokens(payload: dict[str, Any]) -> tuple[int, bool]:
    counts = original_token_counts(payload.get("output"))
    if counts:
        # Wrappers sometimes repeat the same metadata in more than one view.
        return max(counts), True
    return estimated_tokens(visible_output_text(payload.get("output"))), False


def rollout_is_complete(path: Path) -> bool:
    try:
        with path.open("rb") as handle:
            handle.seek(0, 2)
            end = handle.tell()
            if end == 0:
                return False
            position = end - 1
            while position > 0:
                handle.seek(position)
                if handle.read(1) == b"\n" and position < end - 1:
                    break
                position -= 1
            handle.seek(position + (1 if position else 0))
            record = json.loads(handle.readline())
    except (OSError, json.JSONDecodeError):
        return False
    payload = record.get("payload")
    return (
        record.get("type") == "event_msg"
        and isinstance(payload, dict)
        and payload.get("type") in {"task_complete", "task_aborted", "turn_aborted", "task_cancelled"}
    )


def scan_rollouts(
    paths: Iterable[Path],
    hook_text: str,
    additional_hook_texts: Iterable[str] = (),
    include_incomplete: bool = False,
    settled_minutes: float = 10.0,
    record_cutoff: dt.datetime | None = None,
) -> Corpus:
    corpus = Corpus()
    hook_texts = {hook_text, *additional_hook_texts}
    settled_before = dt.datetime.now(dt.timezone.utc).timestamp() - settled_minutes * 60
    calls_by_id: dict[tuple[str, str], ToolCall] = {}
    hook_events_by_id: dict[tuple[str, str, str], HookEvent] = {}
    prompt_events_by_id: dict[tuple[str, str], PromptEvent] = {}
    for path in paths:
        try:
            modified_at = path.stat().st_mtime
        except OSError:
            corpus.incomplete_files_skipped += 1
            continue
        if record_cutoff is None and settled_minutes > 0 and modified_at > settled_before:
            corpus.unsettled_files_skipped += 1
            continue
        if record_cutoff is None and not include_incomplete and not rollout_is_complete(path):
            corpus.incomplete_files_skipped += 1
            continue
        file_counted = False
        session_id = path.stem
        cwd = ""
        hooks_seen: dict[str, int] = defaultdict(int)
        pending: dict[str, ToolCall] = {}
        try:
            handle = path.open("r", encoding="utf-8")
        except OSError as error:
            print(f"warning: cannot read {path}: {error}", file=sys.stderr)
            continue
        with handle:
            for raw_line in handle:
                corpus.lines += 1
                try:
                    record = json.loads(raw_line)
                except json.JSONDecodeError:
                    corpus.malformed_lines += 1
                    continue
                timestamp_raw = record.get("timestamp")
                if not isinstance(timestamp_raw, str):
                    continue
                try:
                    timestamp = parse_timestamp(timestamp_raw)
                except ValueError:
                    corpus.malformed_lines += 1
                    continue
                if record_cutoff is not None and timestamp > record_cutoff:
                    continue
                if not file_counted:
                    corpus.files += 1
                    file_counted = True
                corpus.first_timestamp = min(corpus.first_timestamp or timestamp, timestamp)
                corpus.last_timestamp = max(corpus.last_timestamp or timestamp, timestamp)
                payload = record.get("payload")
                if not isinstance(payload, dict):
                    continue

                if record.get("type") == "session_meta":
                    session_id = str(payload.get("session_id") or payload.get("id") or session_id)
                    cwd = str(payload.get("cwd") or "")
                    corpus.sessions.add(session_id)
                    continue

                payload_type = str(payload.get("type") or "")
                if record.get("type") == "response_item":
                    text = message_text(payload)
                    if payload_type == "message" and payload.get("role") == "developer" and text in hook_texts:
                        hooks_seen[text] += 1
                        metadata = payload.get("internal_chat_message_metadata_passthrough")
                        create_time = metadata.get("create_time") if isinstance(metadata, dict) else None
                        hook_timestamp = (
                            dt.datetime.fromtimestamp(create_time, dt.timezone.utc)
                            if isinstance(create_time, (int, float))
                            else timestamp
                        )
                        message_id = str(payload.get("id") or f"{timestamp.isoformat()}:{record.get('ordinal')}")
                        hook_key = (session_id, message_id, text)
                        hook_event = hook_events_by_id.get(hook_key)
                        if hook_event is None:
                            hook_events_by_id[hook_key] = HookEvent(
                                hook_timestamp, session_id, text, {str(path)}
                            )
                        else:
                            hook_event.rollouts.add(str(path))
                    elif is_tool_call_type(payload_type):
                        call_id = str(payload.get("call_id") or payload.get("id") or f"{path}:{corpus.lines}")
                        command = command_from_payload(payload)
                        key = (session_id, call_id)
                        call = calls_by_id.get(key)
                        if call is None:
                            call = ToolCall(
                                timestamp=timestamp,
                                session_id=session_id,
                                cwd=cwd,
                                call_id=call_id,
                                command=command,
                                category=classify_command(command),
                            )
                            calls_by_id[key] = call
                        pending[call_id] = call
                    elif payload_type.endswith("_call_output"):
                        call_id = output_call_id(payload)
                        call = pending.pop(call_id, None) if call_id else None
                        if call is not None:
                            tokens, exact = output_tokens(payload)
                            if exact or not call.exact_output_tokens:
                                call.output_tokens = tokens
                                call.exact_output_tokens = exact
                                call.output_text = visible_output_text(payload.get("output"))

                if record.get("type") == "event_msg" and payload_type == "token_count":
                    info = payload.get("info") if isinstance(payload.get("info"), dict) else {}
                    usage = info.get("last_token_usage") if isinstance(info.get("last_token_usage"), dict) else {}
                    key = (session_id, timestamp.isoformat())
                    prompt = PromptEvent(
                        timestamp,
                        session_id,
                        dict(hooks_seen),
                        int(usage.get("input_tokens") or 0),
                        int(usage.get("cached_input_tokens") or 0),
                    )
                    old_prompt = prompt_events_by_id.get(key)
                    if old_prompt is None or sum(prompt.hooks_seen_by_text.values()) > sum(
                        old_prompt.hooks_seen_by_text.values()
                    ):
                        prompt_events_by_id[key] = prompt
    corpus.calls = list(calls_by_id.values())
    corpus.hooks = list(hook_events_by_id.values())
    corpus.prompts = list(prompt_events_by_id.values())
    return corpus


OUTLINE_HEADER_RE = re.compile(r"(?m)^# ([^\r\n]+)$")
EXACT_READ_COMMANDS = {"awk", "cat", "grep", "head", "nl", "rg", "ripgrep", "sed", "tail"}


def observed_rewrite_files(call: ToolCall) -> list[str]:
    """Return file headers proving that a non-wot call produced wot output."""
    if not call.command or classify_command(call.command) == "wot":
        return []
    output = call.output_text
    nested = call.output_text
    for _ in range(3):
        try:
            decoded = json.loads(nested)
        except json.JSONDecodeError:
            break
        nested = visible_output_text(decoded)
        output += "\n" + nested
    output = output.replace("\\n", "\n")
    return sorted(
        {
            header
            for header in OUTLINE_HEADER_RE.findall(output)
            if header in call.command
        }
    )


def same_file_read_kind(command: str | None, file: str) -> str | None:
    if not command or file not in command:
        return None
    for segment in split_shell_segments(command):
        if file not in segment:
            continue
        words = unwrap_prefixes(shell_words(segment))
        if not words:
            continue
        executable = Path(words[0]).name
        if executable == "wot":
            return "wot"
        if executable in EXACT_READ_COMMANDS:
            return "exact"
    return None


def build_rewrite_audit(corpus: Corpus, recovery_seconds: float) -> dict[str, Any]:
    calls_by_session: dict[str, list[ToolCall]] = defaultdict(list)
    for call in corpus.calls:
        calls_by_session[call.session_id].append(call)
    for calls in calls_by_session.values():
        calls.sort(key=lambda call: call.timestamp)

    rewrites = []
    recovery_window = dt.timedelta(seconds=recovery_seconds)
    for call in sorted(corpus.calls, key=lambda item: item.timestamp):
        files = observed_rewrite_files(call)
        if not files:
            continue
        exact_recovery = None
        wot_repeat = None
        for later in calls_by_session[call.session_id]:
            if later.timestamp <= call.timestamp:
                continue
            if later.timestamp > call.timestamp + recovery_window:
                break
            for file in files:
                kind = same_file_read_kind(later.command, file)
                if kind == "exact" and exact_recovery is None:
                    exact_recovery = {
                        "timestamp": later.timestamp.isoformat(),
                        "delay_seconds": (later.timestamp - call.timestamp).total_seconds(),
                        "command": later.command,
                    }
                elif kind == "wot" and wot_repeat is None:
                    wot_repeat = {
                        "timestamp": later.timestamp.isoformat(),
                        "delay_seconds": (later.timestamp - call.timestamp).total_seconds(),
                        "command": later.command,
                    }
        rewrites.append(
            {
                "session_id": call.session_id,
                "timestamp": call.timestamp.isoformat(),
                "cwd": call.cwd,
                "command": call.command,
                "files": files,
                "output_tokens": call.output_tokens,
                "exact_output_tokens": call.exact_output_tokens,
                "exact_read_recovery": exact_recovery,
                "explicit_wot_repeat": wot_repeat,
            }
        )

    return {
        "method": {
            "detection": "a recorded non-wot command produced a wot '# <file>' header for a path present in its command",
            "recovery_window_seconds": recovery_seconds,
            "token_note": "tool-output tokens use recorded original_token_count when available and ceil(UTF-8 bytes / 4) otherwise",
        },
        "corpus": {
            "rollout_files": corpus.files,
            "sessions": len(corpus.sessions),
            "tool_calls": len(corpus.calls),
            "incomplete_rollout_files_skipped": corpus.incomplete_files_skipped,
            "unsettled_rollout_files_skipped": corpus.unsettled_files_skipped,
        },
        "summary": {
            "observed_rewrites": len(rewrites),
            "sessions_with_rewrites": len({item["session_id"] for item in rewrites}),
            "rewrite_output_tokens": sum(item["output_tokens"] for item in rewrites),
            "rewrites_with_exact_read_recovery": sum(
                item["exact_read_recovery"] is not None for item in rewrites
            ),
            "rewrites_with_explicit_wot_repeat": sum(
                item["explicit_wot_repeat"] is not None for item in rewrites
            ),
        },
        "rewrites": rewrites,
    }


def rewrite_audit_markdown(report: dict[str, Any]) -> str:
    corpus = report["corpus"]
    summary = report["summary"]
    rows = [
        "# wot hook rewrite audit",
        "",
        f"Scanned {number(corpus['rollout_files'])} rollout files / {number(corpus['sessions'])} sessions "
        f"and {number(corpus['tool_calls'])} tool calls.",
        "",
        f"- Observed rewrites: {number(summary['observed_rewrites'])} across "
        f"{number(summary['sessions_with_rewrites'])} sessions.",
        f"- Rewritten-call output tokens: {number(summary['rewrite_output_tokens'])}.",
        f"- Followed by an exact read of the same file: "
        f"{number(summary['rewrites_with_exact_read_recovery'])}.",
        f"- Followed by an explicit wot repeat: "
        f"{number(summary['rewrites_with_explicit_wot_repeat'])}.",
        "",
        "| Session | Timestamp | Tokens | Exact recovery | wot repeat | Command |",
        "|---|---|---:|---:|---:|---|",
    ]
    for item in report["rewrites"]:
        command = " ".join(item["command"].split())
        if len(command) > 96:
            command = command[:93] + "..."
        rows.append(
            f"| `{item['session_id'][:12]}` | {item['timestamp']} | {number(item['output_tokens'])} | "
            f"{'yes' if item['exact_read_recovery'] else 'no'} | "
            f"{'yes' if item['explicit_wot_repeat'] else 'no'} | `{command}` |"
        )
    rows.extend(
        [
            "",
            "A recovery is observational evidence that the outline was insufficient for the immediate task; "
            "it does not prove that every outline token was wasted.",
            "",
        ]
    )
    return "\n".join(rows)


def discover_rollouts(inputs: list[str]) -> list[Path]:
    paths: list[Path] = []
    for raw in inputs:
        path = Path(raw).expanduser()
        if path.is_dir():
            paths.extend(path.rglob("*.jsonl"))
        elif path.is_file():
            paths.append(path)
        else:
            print(f"warning: input does not exist: {path}", file=sys.stderr)
    return sorted(set(paths))


def summarize_calls(calls: Iterable[ToolCall]) -> dict[str, Any]:
    selected = list(calls)
    shell_calls = [call for call in selected if call.command is not None]
    by_category = {category: [call for call in selected if call.category == category] for category in ("broad", "wot", "other")}
    result: dict[str, Any] = {
        "tool_calls": len(selected),
        "shell_calls": len(shell_calls),
        "exact_output_token_calls": sum(call.exact_output_tokens for call in selected),
    }
    for category, category_calls in by_category.items():
        tokens = sum(call.output_tokens for call in category_calls)
        result[f"{category}_calls"] = len(category_calls)
        result[f"{category}_exact_output_token_calls"] = sum(call.exact_output_tokens for call in category_calls)
        result[f"{category}_output_tokens"] = tokens
        result[f"{category}_mean_output_tokens"] = tokens / len(category_calls) if category_calls else 0.0
        result[f"{category}_median_output_tokens"] = statistics.median(
            call.output_tokens for call in category_calls
        ) if category_calls else 0.0
    result["overview_output_tokens"] = result["broad_output_tokens"] + result["wot_output_tokens"]
    result["overview_tokens_per_100_tool_calls"] = (
        100 * result["overview_output_tokens"] / result["tool_calls"] if result["tool_calls"] else 0.0
    )
    result["overview_tokens_per_100_shell_calls"] = (
        100 * result["overview_output_tokens"] / result["shell_calls"] if result["shell_calls"] else 0.0
    )
    result["broad_calls_per_100_shell_calls"] = (
        100 * result["broad_calls"] / result["shell_calls"] if result["shell_calls"] else 0.0
    )
    result["wot_calls_per_100_shell_calls"] = (
        100 * result["wot_calls"] / result["shell_calls"] if result["shell_calls"] else 0.0
    )
    return result


def matched_cwd_estimate(pre_calls: list[ToolCall], post_calls: list[ToolCall], minimum_calls: int) -> dict[str, Any]:
    pre_by_cwd: dict[str, list[ToolCall]] = defaultdict(list)
    post_by_cwd: dict[str, list[ToolCall]] = defaultdict(list)
    for call in pre_calls:
        pre_by_cwd[call.cwd].append(call)
    for call in post_calls:
        post_by_cwd[call.cwd].append(call)
    matched = sorted(set(pre_by_cwd) & set(post_by_cwd))
    expected = 0.0
    actual = 0
    accepted = []
    for cwd in matched:
        pre = summarize_calls(pre_by_cwd[cwd])
        post = summarize_calls(post_by_cwd[cwd])
        if pre["shell_calls"] < minimum_calls or post["shell_calls"] < minimum_calls:
            continue
        rate = pre["overview_output_tokens"] / pre["shell_calls"]
        expected += rate * post["shell_calls"]
        actual += post["overview_output_tokens"]
        accepted.append(cwd)
    return {
        "minimum_calls_per_side": minimum_calls,
        "matched_cwds": len(accepted),
        "expected_post_overview_tokens": expected,
        "actual_post_overview_tokens": actual,
        "estimated_savings_tokens": expected - actual,
    }


def after_hook_estimate(
    hooks: list[HookEvent],
    calls: list[ToolCall],
    start: dt.datetime,
    end: dt.datetime,
    assumed_broad_tokens: float,
) -> dict[str, Any]:
    calls_by_session: dict[str, list[ToolCall]] = defaultdict(list)
    for call in calls:
        if start <= call.timestamp < end:
            calls_by_session[call.session_id].append(call)
    for session_calls in calls_by_session.values():
        session_calls.sort(key=lambda call: call.timestamp)

    next_calls: dict[tuple[str, str], ToolCall] = {}
    hooks_with_next = 0
    for hook in hooks:
        if not start <= hook.timestamp < end:
            continue
        session_calls = calls_by_session.get(hook.session_id, [])
        timestamps = [call.timestamp for call in session_calls]
        index = bisect.bisect_right(timestamps, hook.timestamp)
        if index < len(session_calls):
            call = session_calls[index]
            hooks_with_next += 1
            next_calls[(call.session_id, call.call_id)] = call

    selected = list(next_calls.values())
    wot_calls = [call for call in selected if call.category == "wot"]
    wot_tokens = sum(call.output_tokens for call in wot_calls)
    substitution = len(wot_calls) * assumed_broad_tokens - wot_tokens
    return {
        "hook_messages_with_a_later_tool_call": hooks_with_next,
        "unique_next_tool_calls": len(selected),
        "unique_next_wot_calls": len(wot_calls),
        "unique_next_broad_calls": sum(call.category == "broad" for call in selected),
        "unique_next_other_calls": sum(call.category == "other" for call in selected),
        "next_call_wot_percent": 100 * len(wot_calls) / len(selected) if selected else 0.0,
        "assumed_displaced_broad_tokens_per_wot": assumed_broad_tokens,
        "next_wot_output_tokens": wot_tokens,
        "estimated_substitution_savings_tokens": substitution,
    }


def paired_representation_estimate(calls: list[ToolCall], start: dt.datetime, end: dt.datetime) -> dict[str, Any]:
    wot_outputs: dict[tuple[str, str], list[int]] = defaultdict(list)
    cat_outputs: dict[tuple[str, str], list[int]] = defaultdict(list)
    for call in calls:
        if not start <= call.timestamp < end:
            continue
        wot_path = single_wot_file(call)
        cat_path = single_cat_file(call)
        if wot_path:
            wot_outputs[(call.session_id, wot_path)].append(call.output_tokens)
        if cat_path:
            cat_outputs[(call.session_id, cat_path)].append(call.output_tokens)
    keys = sorted(set(wot_outputs) & set(cat_outputs))
    wot_tokens = sum(statistics.median(wot_outputs[key]) for key in keys)
    cat_tokens = sum(statistics.median(cat_outputs[key]) for key in keys)
    return {
        "same_session_same_file_pairs": len(keys),
        "paired_wot_output_tokens": wot_tokens,
        "paired_full_cat_output_tokens": cat_tokens,
        "potential_representation_savings_tokens": cat_tokens - wot_tokens,
        "potential_representation_savings_percent": (
            100 * (cat_tokens - wot_tokens) / cat_tokens if cat_tokens else 0.0
        ),
        "actual_combined_output_tokens": cat_tokens + wot_tokens,
        "note": "potential savings compares representations; because both calls occurred, their actual combined output was spent",
    }


def variant_period(
    corpus: Corpus,
    hook_text: str,
    start: dt.datetime,
    end: dt.datetime,
    message_overhead: int,
) -> dict[str, Any]:
    calls = [call for call in corpus.calls if start <= call.timestamp < end]
    hooks = [event for event in corpus.hooks if event.text == hook_text and start <= event.timestamp < end]
    prompts = [event for event in corpus.prompts if start <= event.timestamp < end]
    per_message = estimated_tokens(hook_text) + message_overhead
    summary = summarize_calls(calls)
    exposure = sum(event.hooks_seen_by_text.get(hook_text, 0) * per_message for event in prompts)
    return {
        "start": start.isoformat(),
        "end": end.isoformat(),
        "hook_messages": len(hooks),
        "estimated_tokens_per_message": per_message,
        "estimated_added_context_tokens": len(hooks) * per_message,
        "cumulative_prompt_exposure_upper_estimate": exposure,
        "calls": summary,
        "immediate_response": after_hook_estimate(
            hooks,
            calls,
            start,
            end,
            summary["broad_median_output_tokens"],
        ),
    }


def build_variant_comparison(
    corpus: Corpus,
    old_text: str,
    new_text: str,
    window_days: float,
    message_overhead: int,
) -> dict[str, Any]:
    new_hooks = [event for event in corpus.hooks if event.text == new_text]
    old_hooks = [event for event in corpus.hooks if event.text == old_text]
    if not new_hooks:
        raise ValueError("comparison hook text was found, but the primary hook text was not")
    transition = min(event.timestamp for event in new_hooks)
    requested = dt.timedelta(days=window_days)
    before = transition - corpus.first_timestamp if corpus.first_timestamp else requested
    after = corpus.last_timestamp - transition if corpus.last_timestamp else requested
    window = min(requested, before, after)
    old = variant_period(corpus, old_text, transition - window, transition, message_overhead)
    new = variant_period(corpus, new_text, transition, transition + window, message_overhead)
    old_cost = old["estimated_tokens_per_message"]
    new_cost = new["estimated_tokens_per_message"]
    new_messages = new["hook_messages"]
    new_prompt_units = (
        new["cumulative_prompt_exposure_upper_estimate"] / new_cost if new_cost else 0.0
    )
    return {
        "transition": transition.isoformat(),
        "effective_days_each_side": window.total_seconds() / 86_400,
        "old": old,
        "new": new,
        "history": {
            "old": {
                "hook_messages": len(old_hooks),
                "sessions": len({event.session_id for event in old_hooks}),
                "rollout_files": len(set().union(*(event.rollouts for event in old_hooks))) if old_hooks else 0,
                "first": min(event.timestamp for event in old_hooks).isoformat() if old_hooks else None,
                "last": max(event.timestamp for event in old_hooks).isoformat() if old_hooks else None,
            },
            "new": {
                "hook_messages": len(new_hooks),
                "sessions": len({event.session_id for event in new_hooks}),
                "rollout_files": len(set().union(*(event.rollouts for event in new_hooks))) if new_hooks else 0,
                "first": min(event.timestamp for event in new_hooks).isoformat() if new_hooks else None,
                "last": max(event.timestamp for event in new_hooks).isoformat() if new_hooks else None,
            },
        },
        "wording_only_counterfactual": {
            "estimated_tokens_saved_per_injection": old_cost - new_cost,
            "estimated_cost_reduction_percent": 100 * (old_cost - new_cost) / old_cost if old_cost else 0.0,
            "estimated_old_wording_cost_at_new_message_count": new_messages * old_cost,
            "estimated_new_wording_cost_at_new_message_count": new_messages * new_cost,
            "estimated_added_context_tokens_saved": new_messages * (old_cost - new_cost),
            "cumulative_prompt_exposure_saved_upper_estimate": new_prompt_units * (old_cost - new_cost),
        },
        "note": "wording-only figures hold the new period's injection and prompt-exposure pattern fixed; behavioral period comparisons remain observational",
    }


def build_report(
    corpus: Corpus,
    hook_text: str,
    message_overhead: int,
    window_days: float,
    cutover_override: str | None,
    minimum_cwd_calls: int,
) -> dict[str, Any]:
    primary_hooks = [event for event in corpus.hooks if event.text == hook_text]
    if cutover_override:
        cutover = parse_timestamp(cutover_override)
    elif primary_hooks:
        cutover = min(event.timestamp for event in primary_hooks)
    else:
        raise ValueError("no exact hook messages found; pass --cutover to analyze a known boundary")

    requested_window = dt.timedelta(days=window_days)
    available_before = cutover - corpus.first_timestamp if corpus.first_timestamp else requested_window
    available_after = corpus.last_timestamp - cutover if corpus.last_timestamp else requested_window
    window = min(requested_window, available_before, available_after)
    pre_start, post_end = cutover - window, cutover + window
    pre_calls = [call for call in corpus.calls if pre_start <= call.timestamp < cutover]
    post_calls = [call for call in corpus.calls if cutover <= call.timestamp < post_end]
    pre = summarize_calls(pre_calls)
    post = summarize_calls(post_calls)

    hook_payload_tokens = estimated_tokens(hook_text)
    hook_tokens = hook_payload_tokens + message_overhead
    window_hooks = [event for event in primary_hooks if cutover <= event.timestamp < post_end]
    window_prompts = [event for event in corpus.prompts if cutover <= event.timestamp < post_end]
    added_hook_tokens = len(window_hooks) * hook_tokens
    prompt_exposure = sum(event.hooks_seen_by_text.get(hook_text, 0) * hook_tokens for event in window_prompts)
    measured_input_tokens = sum(event.input_tokens for event in window_prompts)
    measured_cached_tokens = sum(event.cached_input_tokens for event in window_prompts)

    if pre["shell_calls"]:
        expected_post = pre["overview_output_tokens"] / pre["shell_calls"] * post["shell_calls"]
    else:
        expected_post = 0.0
    pooled_savings = expected_post - post["overview_output_tokens"]

    per_wot_delta = pre["broad_mean_output_tokens"] - post["wot_mean_output_tokens"]
    substitution_savings = post["wot_calls"] * per_wot_delta
    after_hook = after_hook_estimate(
        window_hooks,
        post_calls,
        cutover,
        post_end,
        pre["broad_median_output_tokens"],
    )
    paired = paired_representation_estimate(post_calls, cutover, post_end)

    exact_calls = sum(call.exact_output_tokens for call in corpus.calls)
    return {
        "method": {
            "hook_text": hook_text,
            "fallback_token_estimator": "ceil(UTF-8 bytes / 4)",
            "message_overhead_tokens": message_overhead,
            "cutover_basis": "override" if cutover_override else "first exact hook message",
            "causality_warning": "pre/post and substitution savings are estimates, not randomized causal measurements",
        },
        "corpus": {
            "rollout_files": corpus.files,
            "incomplete_rollout_files_skipped": corpus.incomplete_files_skipped,
            "unsettled_rollout_files_skipped": corpus.unsettled_files_skipped,
            "sessions": len(corpus.sessions),
            "jsonl_lines": corpus.lines,
            "malformed_lines": corpus.malformed_lines,
            "first_timestamp": corpus.first_timestamp.isoformat() if corpus.first_timestamp else None,
            "last_timestamp": corpus.last_timestamp.isoformat() if corpus.last_timestamp else None,
            "tool_calls": len(corpus.calls),
            "tool_calls_with_exact_output_tokens": exact_calls,
            "exact_output_coverage_percent": 100 * exact_calls / len(corpus.calls) if corpus.calls else 0.0,
            "hook_messages": len(primary_hooks),
            "sessions_with_hook": len({event.session_id for event in primary_hooks}),
            "rollout_files_with_hook": (
                len(set().union(*(event.rollouts for event in primary_hooks))) if primary_hooks else 0
            ),
            "first_hook_timestamp": min(event.timestamp for event in primary_hooks).isoformat() if primary_hooks else None,
            "last_hook_timestamp": max(event.timestamp for event in primary_hooks).isoformat() if primary_hooks else None,
            "model_prompt_events": len(corpus.prompts),
        },
        "window": {
            "requested_days_each_side": window_days,
            "effective_days_each_side": window.total_seconds() / 86_400,
            "pre_start": pre_start.isoformat(),
            "cutover": cutover.isoformat(),
            "post_end": post_end.isoformat(),
        },
        "hook_cost": {
            "estimated_payload_tokens_per_message": hook_payload_tokens,
            "estimated_tokens_per_injected_message": hook_tokens,
            "post_window_hook_messages": len(window_hooks),
            "post_window_added_context_tokens_lower_bound": added_hook_tokens,
            "post_window_cumulative_prompt_exposure_upper_estimate": prompt_exposure,
            "post_window_measured_input_tokens": measured_input_tokens,
            "post_window_measured_cached_input_tokens": measured_cached_tokens,
            "upper_exposure_percent_of_measured_input": (
                100 * prompt_exposure / measured_input_tokens if measured_input_tokens else 0.0
            ),
            "note": "the upper estimate assumes every prior hook message remains in every later recorded prompt; compaction and cache billing can make actual cost lower",
        },
        "pre": pre,
        "post": post,
        "pooled_pre_post_estimate": {
            "expected_post_overview_tokens_at_pre_rate": expected_post,
            "actual_post_overview_tokens": post["overview_output_tokens"],
            "estimated_savings_tokens": pooled_savings,
            "net_after_added_hook_context_tokens": pooled_savings - added_hook_tokens,
            "net_after_upper_prompt_exposure": pooled_savings - prompt_exposure,
        },
        "matched_cwd_pre_post_estimate": matched_cwd_estimate(pre_calls, post_calls, minimum_cwd_calls),
        "one_for_one_substitution_estimate": {
            "assumption": "each post-window wot call displaced one pre-window mean broad read",
            "pre_mean_broad_read_output_tokens": pre["broad_mean_output_tokens"],
            "post_mean_wot_output_tokens": post["wot_mean_output_tokens"],
            "estimated_savings_per_wot_call": per_wot_delta,
            "estimated_total_savings_tokens": substitution_savings,
            "net_after_added_hook_context_tokens": substitution_savings - added_hook_tokens,
            "net_after_upper_prompt_exposure": substitution_savings - prompt_exposure,
        },
        "immediate_after_hook_estimate": after_hook,
        "paired_representation_estimate": paired,
    }


def number(value: Any) -> str:
    if isinstance(value, float):
        return f"{value:,.1f}"
    if isinstance(value, int):
        return f"{value:,}"
    return str(value)


def markdown_report(report: dict[str, Any]) -> str:
    corpus, window, cost = report["corpus"], report["window"], report["hook_cost"]
    pre, post = report["pre"], report["post"]
    pooled = report["pooled_pre_post_estimate"]
    matched = report["matched_cwd_pre_post_estimate"]
    substitution = report["one_for_one_substitution_estimate"]
    after_hook = report["immediate_after_hook_estimate"]
    paired = report["paired_representation_estimate"]
    rows = [
        "# wot hook token analysis",
        "",
        f"Scanned {number(corpus['rollout_files'])} rollout files / {number(corpus['sessions'])} sessions "
        f"({window['pre_start']} to {window['post_end']}; cutover {window['cutover']}).",
        f"Skipped {number(corpus['incomplete_rollout_files_skipped'])} incomplete and "
        f"{number(corpus['unsettled_rollout_files_skipped'])} recently modified rollout files.",
        "",
        "## Corpus and hook cost",
        "",
        f"- Tool calls: {number(corpus['tool_calls'])}; exact tool-output token coverage: "
        f"{number(corpus['exact_output_coverage_percent'])}%.",
        f"- Exact hook messages: {number(corpus['hook_messages'])} across "
        f"{number(corpus['sessions_with_hook'])} sessions.",
        f"- Estimated cost per injected message: {number(cost['estimated_tokens_per_injected_message'])} tokens "
        f"({number(cost['estimated_payload_tokens_per_message'])} payload + configured framing).",
        f"- Post-window added-context lower bound: {number(cost['post_window_added_context_tokens_lower_bound'])} tokens.",
        f"- Post-window cumulative prompt-exposure upper estimate: "
        f"{number(cost['post_window_cumulative_prompt_exposure_upper_estimate'])} tokens "
        f"({number(cost['upper_exposure_percent_of_measured_input'])}% of measured input).",
        "",
        "## Equal-window observations",
        "",
        "| Metric | Before | After |",
        "|---|---:|---:|",
        f"| Tool calls | {number(pre['tool_calls'])} | {number(post['tool_calls'])} |",
        f"| Shell calls | {number(pre['shell_calls'])} | {number(post['shell_calls'])} |",
        f"| Broad reads | {number(pre['broad_calls'])} | {number(post['broad_calls'])} |",
        f"| Broad reads / 100 shell calls | {number(pre['broad_calls_per_100_shell_calls'])} | "
        f"{number(post['broad_calls_per_100_shell_calls'])} |",
        f"| Broad-read output tokens | {number(pre['broad_output_tokens'])} | {number(post['broad_output_tokens'])} |",
        f"| wot calls | {number(pre['wot_calls'])} | {number(post['wot_calls'])} |",
        f"| wot calls / 100 shell calls | {number(pre['wot_calls_per_100_shell_calls'])} | "
        f"{number(post['wot_calls_per_100_shell_calls'])} |",
        f"| wot output tokens | {number(pre['wot_output_tokens'])} | {number(post['wot_output_tokens'])} |",
        f"| Overview tokens / 100 tool calls | {number(pre['overview_tokens_per_100_tool_calls'])} | "
        f"{number(post['overview_tokens_per_100_tool_calls'])} |",
        f"| Overview tokens / 100 shell calls | {number(pre['overview_tokens_per_100_shell_calls'])} | "
        f"{number(post['overview_tokens_per_100_shell_calls'])} |",
        "",
        "## Estimates",
        "",
        f"- Pooled pre/post savings: {number(pooled['estimated_savings_tokens'])} tokens; "
        f"net after lower-bound hook cost: {number(pooled['net_after_added_hook_context_tokens'])}; "
        f"net after upper prompt exposure: {number(pooled['net_after_upper_prompt_exposure'])}.",
        f"- Matched-CWD pre/post savings ({number(matched['matched_cwds'])} CWDs): "
        f"{number(matched['estimated_savings_tokens'])} tokens.",
        f"- One-for-one substitution savings: {number(substitution['estimated_total_savings_tokens'])} tokens "
        f"({number(substitution['estimated_savings_per_wot_call'])} per post-window wot call); "
        f"net after lower-bound hook cost: {number(substitution['net_after_added_hook_context_tokens'])}.",
        f"- Immediate response: {number(after_hook['unique_next_wot_calls'])} of "
        f"{number(after_hook['unique_next_tool_calls'])} unique next tool calls were wot "
        f"({number(after_hook['next_call_wot_percent'])}%); median-baseline substitution estimate: "
        f"{number(after_hook['estimated_substitution_savings_tokens'])} tokens.",
        f"- Same-session/same-file representation pairs: {number(paired['same_session_same_file_pairs'])}; "
        f"potential compression: {number(paired['potential_representation_savings_tokens'])} tokens "
        f"({number(paired['potential_representation_savings_percent'])}%). Because both calls occurred, actual "
        f"combined output was {number(paired['actual_combined_output_tokens'])} tokens.",
        "",
        "The pre/post and one-for-one figures are observational estimates. The transcript does not contain the "
        "counterfactual tool call an agent would have made without the reminder, and prompt caching/compaction is "
        "not attributable per message.",
        "",
    ]
    comparison = report.get("variant_comparison")
    if comparison:
        old, new = comparison["old"], comparison["new"]
        old_history, new_history = comparison["history"]["old"], comparison["history"]["new"]
        old_calls, new_calls = old["calls"], new["calls"]
        old_response, new_response = old["immediate_response"], new["immediate_response"]
        wording = comparison["wording_only_counterfactual"]
        rows.extend(
            [
                "## Hook wording comparison",
                "",
                f"Adjacent periods of {number(comparison['effective_days_each_side'])} days around "
                f"{comparison['transition']}:",
                f"Previous wording: {number(old_history['hook_messages'])} deduplicated messages across "
                f"{number(old_history['rollout_files'])} rollout files ({old_history['first']} to "
                f"{old_history['last']}). Primary wording: {number(new_history['hook_messages'])} messages "
                f"across {number(new_history['rollout_files'])} rollout files ({new_history['first']} to "
                f"{new_history['last']}).",
                "",
                "| Metric | Previous wording | Primary wording |",
                "|---|---:|---:|",
                f"| Estimated tokens / injection | {number(old['estimated_tokens_per_message'])} | "
                f"{number(new['estimated_tokens_per_message'])} |",
                f"| Hook messages | {number(old['hook_messages'])} | {number(new['hook_messages'])} |",
                f"| Added context tokens | {number(old['estimated_added_context_tokens'])} | "
                f"{number(new['estimated_added_context_tokens'])} |",
                f"| Next-call wot rate | {number(old_response['next_call_wot_percent'])}% | "
                f"{number(new_response['next_call_wot_percent'])}% |",
                f"| Broad reads / 100 shell calls | {number(old_calls['broad_calls_per_100_shell_calls'])} | "
                f"{number(new_calls['broad_calls_per_100_shell_calls'])} |",
                f"| wot calls / 100 shell calls | {number(old_calls['wot_calls_per_100_shell_calls'])} | "
                f"{number(new_calls['wot_calls_per_100_shell_calls'])} |",
                f"| Overview tokens / 100 shell calls | "
                f"{number(old_calls['overview_tokens_per_100_shell_calls'])} | "
                f"{number(new_calls['overview_tokens_per_100_shell_calls'])} |",
                "",
                f"Holding the primary period's injection pattern fixed, the shorter wording saved an estimated "
                f"{number(wording['estimated_added_context_tokens_saved'])} added-context tokens "
                f"({number(wording['estimated_cost_reduction_percent'])}% per injection). Its upper logical "
                f"prompt-exposure saving is {number(wording['cumulative_prompt_exposure_saved_upper_estimate'])} "
                "tokens; most repeated exposure may be cached.",
                "",
            ]
        )
    return "\n".join(rows)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "inputs",
        nargs="+",
        help="Rollout JSONL files or directories (directories are scanned recursively)",
    )
    parser.add_argument("--hook-text", default=DEFAULT_HOOK_TEXT)
    parser.add_argument(
        "--compare-hook-text",
        help="Compare this previous exact hook text with --hook-text in adjacent equal windows",
    )
    parser.add_argument("--message-overhead-tokens", type=int, default=DEFAULT_MESSAGE_OVERHEAD_TOKENS)
    parser.add_argument("--window-days", type=float, default=14.0)
    parser.add_argument("--cutover", help="ISO-8601 cutover; defaults to first exact hook message")
    parser.add_argument("--minimum-cwd-calls", type=int, default=10)
    parser.add_argument("--include-incomplete", action="store_true", help="Include active or aborted rollout tails")
    parser.add_argument(
        "--settled-minutes",
        type=float,
        default=10.0,
        help="Exclude files modified this recently to avoid racing active rollouts (default: 10)",
    )
    parser.add_argument(
        "--record-cutoff",
        help="Ignore records after this ISO-8601 timestamp for a reproducible historical snapshot",
    )
    parser.add_argument(
        "--audit-rewrites",
        action="store_true",
        help="Audit observed silent command rewrites and immediate same-file recovery reads",
    )
    parser.add_argument(
        "--recovery-seconds",
        type=float,
        default=30.0,
        help="Same-session window for rewrite recovery reads (default: 30)",
    )
    parser.add_argument("--format", choices=("markdown", "json"), default="markdown")
    parser.add_argument("--output", type=Path, help="Write the report to this path instead of stdout")
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    paths = discover_rollouts(args.inputs)
    if not paths:
        print("error: no rollout JSONL files found", file=sys.stderr)
        return 2
    record_cutoff = parse_timestamp(args.record_cutoff) if args.record_cutoff else None
    corpus = scan_rollouts(
        paths,
        args.hook_text,
        additional_hook_texts=([args.compare_hook_text] if args.compare_hook_text else []),
        include_incomplete=args.include_incomplete,
        settled_minutes=args.settled_minutes,
        record_cutoff=record_cutoff,
    )
    if args.audit_rewrites:
        report = build_rewrite_audit(corpus, args.recovery_seconds)
        rendered = (
            json.dumps(report, indent=2, sort_keys=True) + "\n"
            if args.format == "json"
            else rewrite_audit_markdown(report)
        )
        if args.output:
            args.output.write_text(rendered, encoding="utf-8")
        else:
            sys.stdout.write(rendered)
        return 0
    try:
        report = build_report(
            corpus,
            args.hook_text,
            args.message_overhead_tokens,
            args.window_days,
            args.cutover,
            args.minimum_cwd_calls,
        )
        if args.compare_hook_text:
            report["variant_comparison"] = build_variant_comparison(
                corpus,
                args.compare_hook_text,
                args.hook_text,
                args.window_days,
                args.message_overhead_tokens,
            )
    except ValueError as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    rendered = json.dumps(report, indent=2, sort_keys=True) + "\n" if args.format == "json" else markdown_report(report)
    if args.output:
        args.output.write_text(rendered, encoding="utf-8")
    else:
        sys.stdout.write(rendered)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

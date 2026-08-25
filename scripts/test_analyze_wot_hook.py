#!/usr/bin/env python3

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("analyze_wot_hook.py")
SPEC = importlib.util.spec_from_file_location("analyze_wot_hook", SCRIPT)
assert SPEC and SPEC.loader
ANALYZER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = ANALYZER
SPEC.loader.exec_module(ANALYZER)


def record(timestamp, ordinal, record_type, payload):
    return {"timestamp": timestamp, "ordinal": ordinal, "type": record_type, "payload": payload}


class AnalyzerTest(unittest.TestCase):
    def test_classifies_wot_and_broad_reads(self):
        self.assertEqual(ANALYZER.classify_command("wot src/lib.rs"), "wot")
        self.assertEqual(ANALYZER.classify_command("sed -n '1,120p' src/lib.rs"), "broad")
        self.assertEqual(ANALYZER.classify_command("rg -n 'symbol' src/lib.rs"), "other")

    def test_scans_exact_counts_and_builds_pre_post_estimate(self):
        old_hook_text = "A much longer historical wot hook reminder used for comparison."
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "rollout.jsonl"
            records = [
                record("2026-01-01T00:00:00Z", 0, "session_meta", {"id": "s1", "cwd": "/repo"}),
                record(
                    "2026-01-01T00:00:00.500Z",
                    1,
                    "response_item",
                    {"type": "message", "role": "developer", "content": [{"type": "input_text", "text": old_hook_text}]},
                ),
                record(
                    "2026-01-01T00:00:01Z",
                    1,
                    "response_item",
                    {"type": "custom_tool_call", "call_id": "pre", "input": 'await tools.exec_command({"cmd":"cat src/lib.rs"})'},
                ),
                record(
                    "2026-01-01T00:00:02Z",
                    2,
                    "response_item",
                    {"type": "custom_tool_call_output", "call_id": "pre", "output": [{"text": '{"original_token_count":100,"output":"x"}'}]},
                ),
                record(
                    "2026-01-02T00:00:00Z",
                    3,
                    "response_item",
                    {"type": "message", "role": "developer", "content": [{"type": "input_text", "text": ANALYZER.DEFAULT_HOOK_TEXT}]},
                ),
                record(
                    "2026-01-02T00:00:01Z",
                    4,
                    "response_item",
                    {"type": "custom_tool_call", "call_id": "post", "input": 'await tools.exec_command({"cmd":"wot src/lib.rs"})'},
                ),
                record(
                    "2026-01-02T00:00:02Z",
                    5,
                    "response_item",
                    {"type": "custom_tool_call_output", "call_id": "post", "output": [{"text": '{"original_token_count":10,"output":"y"}'}]},
                ),
                record(
                    "2026-01-03T00:00:00Z",
                    6,
                    "event_msg",
                    {"type": "token_count", "info": {"last_token_usage": {"input_tokens": 50}}},
                ),
            ]
            path.write_text("".join(json.dumps(item) + "\n" for item in records), encoding="utf-8")
            corpus = ANALYZER.scan_rollouts(
                [path],
                ANALYZER.DEFAULT_HOOK_TEXT,
                additional_hook_texts=[old_hook_text],
                include_incomplete=True,
                settled_minutes=0,
            )
            report = ANALYZER.build_report(corpus, ANALYZER.DEFAULT_HOOK_TEXT, 4, 2, None, 1)
            comparison = ANALYZER.build_variant_comparison(
                corpus, old_hook_text, ANALYZER.DEFAULT_HOOK_TEXT, 2, 4
            )

        self.assertEqual(report["pre"]["broad_output_tokens"], 100)
        self.assertEqual(report["post"]["wot_output_tokens"], 10)
        self.assertEqual(report["pooled_pre_post_estimate"]["estimated_savings_tokens"], 90)
        self.assertEqual(report["one_for_one_substitution_estimate"]["estimated_total_savings_tokens"], 90)
        self.assertGreater(report["hook_cost"]["post_window_added_context_tokens_lower_bound"], 0)
        self.assertEqual(comparison["old"]["hook_messages"], 1)
        self.assertEqual(comparison["new"]["hook_messages"], 1)
        self.assertGreater(
            comparison["wording_only_counterfactual"]["estimated_added_context_tokens_saved"], 0
        )

    def test_audits_observed_rewrite_and_same_file_recovery(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "rollout.jsonl"
            outline_output = json.dumps(
                {
                    "original_token_count": 12,
                    "output": "# docs/a.md\n- A [L1-L200]\n",
                }
            )
            records = [
                record("2026-01-01T00:00:00Z", 0, "session_meta", {"id": "s1", "cwd": "/repo"}),
                record(
                    "2026-01-01T00:00:01Z",
                    1,
                    "response_item",
                    {
                        "type": "custom_tool_call",
                        "call_id": "rewrite",
                        "input": 'await tools.exec_command({"cmd":"sed -n \'1,120p\' docs/a.md"})',
                    },
                ),
                record(
                    "2026-01-01T00:00:02Z",
                    2,
                    "response_item",
                    {
                        "type": "custom_tool_call_output",
                        "call_id": "rewrite",
                        "output": [{"text": outline_output}],
                    },
                ),
                record(
                    "2026-01-01T00:00:10Z",
                    3,
                    "response_item",
                    {
                        "type": "custom_tool_call",
                        "call_id": "recovery",
                        "input": 'await tools.exec_command({"cmd":"awk \'{print}\' docs/a.md"})',
                    },
                ),
            ]
            path.write_text("".join(json.dumps(item) + "\n" for item in records), encoding="utf-8")
            corpus = ANALYZER.scan_rollouts(
                [path],
                ANALYZER.DEFAULT_HOOK_TEXT,
                include_incomplete=True,
                settled_minutes=0,
            )
            report = ANALYZER.build_rewrite_audit(corpus, recovery_seconds=30)

        self.assertEqual(report["summary"]["observed_rewrites"], 1)
        self.assertEqual(report["summary"]["rewrite_output_tokens"], 12)
        self.assertEqual(report["summary"]["rewrites_with_exact_read_recovery"], 1)
        self.assertEqual(report["rewrites"][0]["files"], ["docs/a.md"])


if __name__ == "__main__":
    unittest.main()

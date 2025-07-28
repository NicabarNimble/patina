#!/usr/bin/env python3
"""Parse Claude JSONL session into readable markdown."""

import json
import sys
from datetime import datetime
from collections import defaultdict

def parse_timestamp(ts_str):
    """Convert ISO timestamp to readable format."""
    dt = datetime.fromisoformat(ts_str.replace('Z', '+00:00'))
    return dt.strftime('%H:%M:%S')

def extract_message_text(content):
    """Extract text from various content formats."""
    if isinstance(content, str):
        # Remove command tags
        text = content.replace('<command-message>', '').replace('</command-message>', '')
        text = text.replace('<command-name>', '').replace('</command-name>', '')
        text = text.replace('<command-args>', '').replace('</command-args>', '')
        return text.strip()
    elif isinstance(content, list):
        for item in content:
            if isinstance(item, dict) and item.get('type') == 'text':
                return item.get('text', '').strip()
    return ""

def parse_jsonl(file_path):
    """Parse JSONL file and extract key events."""
    events = []
    tool_uses = defaultdict(int)
    files_modified = set()
    
    with open(file_path, 'r') as f:
        for line in f:
            try:
                event = json.loads(line)
                timestamp = event.get('timestamp', '')
                event_type = event.get('type', '')
                
                if event_type == 'user':
                    text = extract_message_text(event.get('message', {}).get('content', ''))
                    if text and not text.startswith('is running…'):
                        events.append({
                            'time': parse_timestamp(timestamp),
                            'type': 'user',
                            'text': text[:100] + '...' if len(text) > 100 else text
                        })
                
                elif event_type == 'assistant':
                    content = event.get('message', {}).get('content', [])
                    for item in content if isinstance(content, list) else []:
                        if item.get('type') == 'tool_use':
                            tool_name = item.get('name', '')
                            tool_uses[tool_name] += 1
                            
                            # Track file modifications
                            if tool_name in ['Edit', 'Write', 'MultiEdit']:
                                file_path = item.get('input', {}).get('file_path', '')
                                if file_path:
                                    files_modified.add(file_path)
                            
                            events.append({
                                'time': parse_timestamp(timestamp),
                                'type': 'tool',
                                'tool': tool_name,
                                'details': item.get('input', {})
                            })
                        elif item.get('type') == 'text':
                            text = item.get('text', '').strip()
                            if text:  # Capture ALL assistant responses
                                events.append({
                                    'time': parse_timestamp(timestamp),
                                    'type': 'assistant',
                                    'text': text
                                })
                
            except json.JSONDecodeError:
                continue
    
    return events, tool_uses, files_modified

def generate_markdown(events, tool_uses, files_modified):
    """Generate readable markdown from parsed events."""
    if not events:
        return "No events found in session."
    
    # Extract session info
    start_time = events[0]['time']
    end_time = events[-1]['time']
    
    md = f"""# Claude Session Summary

**Date**: July 27, 2025
**Duration**: {start_time} - {end_time}

## Session Overview

This session focused on redesigning Patina's session management system.

## Timeline

"""
    
    # Group events by rough time periods
    for event in events:
        if event['type'] == 'user':
            md += f"\n### [{event['time']}] User\n> {event['text']}\n"
        elif event['type'] == 'assistant':
            # Format longer responses nicely
            if len(event['text']) > 200:
                # Show first paragraph or 200 chars
                preview = event['text'][:200] + "..."
                md += f"\n**Claude**: {preview}\n"
            else:
                md += f"\n**Claude**: {event['text']}\n"
        elif event['type'] == 'tool':
            if event['tool'] == 'Bash':
                cmd = event['details'].get('command', '')
                md += f"- 🔧 Executed: `{cmd[:80]}...`\n" if len(cmd) > 80 else f"- 🔧 Executed: `{cmd}`\n"
            elif event['tool'] in ['Edit', 'Write', 'MultiEdit']:
                file_path = event['details'].get('file_path', '')
                md += f"- 📝 Modified: `{file_path.split('/')[-1]}`\n"
            elif event['tool'] == 'Read':
                file_path = event['details'].get('file_path', '')
                md += f"- 👁️ Read: `{file_path.split('/')[-1]}`\n"
            elif event['tool'] == 'Grep':
                pattern = event['details'].get('pattern', '')
                md += f"- 🔍 Searched for: `{pattern}`\n"
    
    # Summary statistics
    md += f"\n## Session Statistics\n\n"
    md += f"- **Total interactions**: {len([e for e in events if e['type'] == 'user'])}\n"
    md += f"- **Files modified**: {len(files_modified)}\n"
    md += f"\n### Tool Usage:\n"
    for tool, count in sorted(tool_uses.items(), key=lambda x: x[1], reverse=True):
        md += f"- {tool}: {count} times\n"
    
    # Files modified
    if files_modified:
        md += f"\n## Files Modified\n\n"
        for file_path in sorted(files_modified):
            md += f"- `{file_path}`\n"
    
    return md

if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("Usage: python parse_session.py <jsonl_file>")
        sys.exit(1)
    
    events, tool_uses, files_modified = parse_jsonl(sys.argv[1])
    markdown = generate_markdown(events, tool_uses, files_modified)
    print(markdown)
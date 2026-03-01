#!/usr/bin/env python3
"""
Update MCP server to use RwLock instead of Mutex.

Replaces all occurrences of:
  let engine = self.engine.lock().await;
with:
  let engine = self.engine.read().unwrap();
"""

import re
from pathlib import Path

def update_tools_file():
    """Update crates/omni-mcp/src/tools.rs"""
    file_path = Path("crates/omni-mcp/src/tools.rs")
    
    if not file_path.exists():
        print(f"Error: {file_path} not found")
        return False
    
    content = file_path.read_text(encoding='utf-8')
    original_content = content
    
    # Replace all occurrences of engine.lock().await with engine.read().unwrap()
    pattern = r'let engine = self\.engine\.lock\(\)\.await;'
    replacement = r'let engine = self.engine.read().unwrap();'
    
    content = re.sub(pattern, replacement, content)
    
    # Count replacements
    count = len(re.findall(pattern, original_content))
    
    if content != original_content:
        file_path.write_text(content, encoding='utf-8')
        print(f"✅ Updated {file_path}: {count} replacements")
        return True
    else:
        print(f"⚠️  No changes needed in {file_path}")
        return False

def main():
    print("Updating MCP server to use RwLock...")
    print()
    
    success = update_tools_file()
    
    print()
    if success:
        print("✅ All updates complete!")
        print()
        print("Next steps:")
        print("1. cargo build -p omni-mcp")
        print("2. cargo test -p omni-mcp")
        print("3. cargo run -p omni-mcp -- --repo .")
    else:
        print("⚠️  No updates were needed")

if __name__ == "__main__":
    main()

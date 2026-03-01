#!/usr/bin/env python3
"""
Fix remaining self.conn references in index/mod.rs
"""

import re

file_path = "crates/omni-core/src/index/mod.rs"

with open(file_path, 'r') as f:
    content = f.read()

# Replace all remaining self.conn references
content = re.sub(r'self\.conn\.', 'conn.', content)
content = re.sub(r'self\.conn,', 'conn,', content)
content = re.sub(r'&self\.conn', '&conn', content)

# Fix methods that still reference self.conn directly
# Pattern: find lines with "self.conn" that aren't in comments
lines = content.split('\n')
fixed_lines = []
for line in lines:
    if 'self.conn' in line and not line.strip().startswith('//'):
        line = line.replace('self.conn', 'conn')
    fixed_lines.append(line)

content = '\n'.join(fixed_lines)

with open(file_path, 'w') as f:
    f.write(content)

print("✓ Fixed all remaining self.conn references")

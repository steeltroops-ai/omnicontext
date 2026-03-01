#!/usr/bin/env python3
"""
Fix multiline self.conn references
"""

import re

file_path = "crates/omni-core/src/index/mod.rs"

with open(file_path, 'r') as f:
    content = f.read()

# Fix pattern: self\n            .conn
content = re.sub(r'= self\s+\.conn', '= conn', content)
content = re.sub(r'let \w+ = self\s+\.conn', lambda m: m.group(0).replace('self\n            .conn', 'conn'), content)

# More aggressive: any self followed by .conn on next line
content = re.sub(r'self\s*\n\s*\.conn', 'conn', content)

with open(file_path, 'w') as f:
    f.write(content)

print("✓ Fixed multiline self.conn references")

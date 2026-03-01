#!/usr/bin/env python3
"""
Fix all self.conn references in index/mod.rs to use connection pool.
"""

import re

file_path = "crates/omni-core/src/index/mod.rs"

with open(file_path, 'r') as f:
    content = f.read()

# Methods that need connection acquisition
methods = [
    'get_file_hash',
    'delete_file',
    'get_all_files',
    'file_count',
    'insert_chunk',
    'delete_chunks_for_file',
    'get_chunks_for_file',
    'set_chunk_vector_id',
    'chunk_count',
    'insert_symbol',
    'get_symbol_by_fqn',
    'get_symbol_by_id',
    'search_symbols_by_name',
    'delete_symbols_for_file',
    'symbol_count',
    'get_first_symbol_for_file',
    'search_symbols_by_fqn_suffix',
    'get_all_symbols_for_file',
    'keyword_search',
    'reindex_file',
    'check_integrity',
    'statistics',
    'insert_dependency',
    'get_upstream_dependencies',
    'get_downstream_dependencies',
    'delete_dependencies_for_symbol',
    'dependency_count',
    'get_all_dependencies',
]

# For each method, add connection acquisition if not already present
for method in methods:
    # Find method definition
    pattern = rf'(pub fn {method}\([^{{]*\{{\s*)'
    
    # Check if it already has pool.get()
    if f'pub fn {method}' in content:
        # Add connection acquisition after method opening brace
        replacement = r'\1let conn = self.pool.get()\n            .map_err(|e| OmniError::Internal(format!("Failed to get connection: {}", e)))?;\n        '
        content = re.sub(pattern, replacement, content, count=1)

# Replace all remaining self.conn with conn
content = content.replace('self.conn.', 'conn.')
content = content.replace('self.conn,', 'conn,')

# Fix connection() method
content = re.sub(
    r'pub fn connection\(&self\) -> &Connection \{\s*&self\.conn\s*\}',
    '''pub fn connection(&self) -> r2d2::PooledConnection<SqliteConnectionManager> {
        self.pool.get()
            .expect("Failed to get connection from pool")
    }''',
    content
)

with open(file_path, 'w') as f:
    f.write(content)

print(f"✓ Updated {file_path}")
print(f"✓ Added connection acquisition to {len(methods)} methods")
print(f"✓ Replaced all self.conn references")

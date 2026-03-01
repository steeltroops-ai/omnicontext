#!/bin/bash
# Script to update all self.conn references to use connection pool

FILE="crates/omni-core/src/index/mod.rs"

# Replace all self.conn with pool.get()
sed -i 's/self\.conn\./conn./g' "$FILE"
sed -i 's/self\.conn,/conn,/g' "$FILE"

# Add connection acquisition at start of each method
# This is a simplified approach - manual review needed

echo "Updated $FILE to use connection pool"
echo "Manual review required for:"
echo "1. Add 'let conn = self.pool.get()?' at start of each method"
echo "2. Update connection() method to return pool reference"
echo "3. Test all methods work correctly"

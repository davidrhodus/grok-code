#!/usr/bin/env python3
"""
Example plugin script for grok-code
Counts words in files with various options
"""

import json
import sys
import os

def count_words(file_path, ignore_case=False, pattern=None):
    """Count words in a file with optional filtering"""
    try:
        with open(file_path, 'r') as f:
            content = f.read()
            
        if ignore_case:
            content = content.lower()
            
        words = content.split()
        
        if pattern:
            if ignore_case:
                pattern = pattern.lower()
            words = [w for w in words if pattern in w]
            
        return {
            "success": True,
            "file": file_path,
            "word_count": len(words),
            "pattern": pattern,
            "message": f"Found {len(words)} words" + (f" containing '{pattern}'" if pattern else "")
        }
    except Exception as e:
        return {
            "success": False,
            "error": str(e),
            "file": file_path
        }

def main():
    # Read arguments from stdin (passed by grok-code)
    try:
        args_json = sys.stdin.read()
        args = json.loads(args_json) if args_json else {}
    except:
        args = {}
    
    # Extract parameters
    file_path = args.get('file', '')
    ignore_case = args.get('ignore_case', False)
    pattern = args.get('pattern', None)
    
    if not file_path:
        print(json.dumps({
            "error": "No file specified",
            "usage": "Provide 'file' parameter with path to file"
        }))
        sys.exit(1)
    
    # Perform word count
    result = count_words(file_path, ignore_case, pattern)
    
    # Output result as JSON
    print(json.dumps(result, indent=2))

if __name__ == "__main__":
    main() 
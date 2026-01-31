#!/bin/bash
set -e

# compiling bit...
echo "🔨 Building cs..."
cargo build --quiet

# where's the binary?
CS="./target/debug/cs"
CLI_JSON=".cawa_cfg.json"

# fresh start - wipe old config
rm -f "$CLI_JSON"

echo "🐙 Testing cs functionality..."

# try add
echo -n "  Testing 'add'..."
$CS add hello "echo Hello World" > /dev/null
grep -q "hello" "$CLI_JSON"
echo "✅"

# try list
echo -n "  Testing 'list'..."
$CS list | grep -q "hello"
echo "✅"

# run it
echo -n "  Testing 'execution'..."
OUTPUT=$($CS hello)
if [[ "$OUTPUT" == *"Hello World"* ]]; then
    echo "✅"
else
    echo "❌ (Output: $OUTPUT)"
    exit 1
fi

# nuke it
echo -n "  Testing 'remove'..."
$CS remove hello > /dev/null
if grep -q "hello" "$CLI_JSON"; then
    echo "❌"
    exit 1
else
    echo "✅"
fi

# parallel toggle test
echo -n "  Testing 'parallel'..."
$CS add -p par "sleep 1 && echo finished_1" "sleep 1 && echo finished_2" > /dev/null
START=$(date +%s)
OUTPUT=$($CS par)
END=$(date +%s)
DURATION=$((END - START))

if [[ "$OUTPUT" == *"finished_1"* && "$OUTPUT" == *"finished_2"* ]]; then
    if [[ "$OUTPUT" == *"⏱️"* ]]; then
         # timing should be ghosted by default
         echo "❌ (Timing showed up but should be disabled)"
         exit 1
    fi
    
    # enable timing manual override - serde is tricky sometimes
    echo '{ "enable_timing": true, "aliases": { "par": ["sleep 1 && echo finished_1", "sleep 1 && echo finished_2"] } }' > "$CLI_JSON"
    
    OUTPUT_TIMED=$($CS par)
    if [[ "$OUTPUT_TIMED" == *"⏱️"* ]]; then
          echo "✅ (Default hidden, Enabled visible)"
    else
          echo "❌ (Timing failed to show after enabling)"
          exit 1
    fi

else
    echo "❌ (Output: $OUTPUT)"
    exit 1
fi

echo "🎉 All manual tests passed!"
rm -f "$CLI_JSON"

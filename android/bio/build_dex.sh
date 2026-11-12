#!/usr/bin/env bash
# Rebuilds classes.dex from BioAuth.java (only needed when the Java source changes).
# Requires: JDK 17+, Android SDK platform 35 and build-tools (d8).
set -euo pipefail
cd "$(dirname "$0")"
SDK="${ANDROID_HOME:-$HOME/Library/Android/sdk}"
ANDROID_JAR="$SDK/platforms/android-35/android.jar"
D8="$(ls -d "$SDK"/build-tools/*/ | sort -V | tail -1)d8"
rm -rf out && mkdir -p out/classes
javac -source 1.8 -target 1.8 -Xlint:-options -bootclasspath "$ANDROID_JAR" \
  -d out/classes src/com/photoexchanger/bio/BioAuth.java
"$D8" --min-api 26 --lib "$ANDROID_JAR" --output out $(find out/classes -name '*.class')
cp out/classes.dex classes.dex
rm -rf out
echo "classes.dex: $(wc -c < classes.dex) bytes"

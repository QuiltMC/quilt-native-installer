#!/usr/bin/env bash
# Copyright (C) 2022 kb1000
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#         http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

echo "Packaging $1 as $2 with app id $3"

name="${2##*/}"

mkdir -p $2.app/Contents/MacOS
rm -f $2.app/Contents/MacOS/$name
ln $1 $2.app/Contents/MacOS/$name
(
    echo '<?xml version="1.0" encoding="UTF-8"?>'
    echo '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">'
    echo "<plist version=\"1.0\">
    <dict>
        <key>CFBundleExecutable</key>
        <string>$name</string>
        <key>CFBundleIdentifier</key>
        <string>$3</string>
        <key>CFBundleInfoDictionaryVersion</key>
        <string>6.0</string>
        <key>CFBundleName</key>
        <string>$name</string>
        <key>CFBundlePackageType</key>
        <string>APPL</string>
        <key>CFBundleShortVersionString</key>
        <string>1.0</string>
        <key>CFBundleSupportedPlatforms</key>
        <array>
                <string>MacOSX</string>
        </array>
        <key>CFBundleVersion</key>
        <string>1</string>
    </dict>
</plist>"
)>$2.app/Contents/Info.plist

<!-- SPDX-License-Identifier: Apache-2.0 -->
<!-- The release page's template (RELEASE.md). The release run fills each
     {placeholder}. Every sentence below is a placeholder (en) until
     language-culture finalises it; the facts are RELEASE.md's. -->
# InterWeave for Android {version}

Download: [interweave-{version}-arm64-v8a.apk]({apk_url})

- SHA-256 of the file: `{apk_sha256}`
- Signing certificate, SHA-256 (the same for every release):
  `{cert_sha256}`

## What is new

{release_notes}

## Installing and updating

InterWeave is not in any app store. Install it by opening the downloaded
file on your phone, after allowing your browser or file manager to
install apps. To update, download the new release from this page and
install it over the installed app: your messages and identity stay. The
app does not check for updates itself.

Before installing, you can check the download: its SHA-256 must match
the one above, and `apksigner verify --print-certs` must show the
signing certificate above. If either differs, do not install it.

## Made with Slint

[![Made with Slint](https://raw.githubusercontent.com/slint-ui/slint/v1.18.1/logo/MadeWithSlint-logo-light.svg)](https://slint.dev)

InterWeave's interface is built with [Slint](https://slint.dev), used
under the Slint Royalty-free License 2.0.

## Third-party notices

The app's native library includes Skia and the libraries it bundles,
each under its own licence: Skia and skcms (BSD-3-Clause),
libjpeg-turbo (IJG and BSD-style), FreeType (FreeType License),
libpng (PNG Reference Library License version 2), zlib (zlib License),
Expat (MIT), Wuffs (Apache-2.0), Vulkan Memory Allocator (MIT) and
cpu_features (BSD-2-Clause). Portions of this software are copyright
© The FreeType Project (www.freetype.org). All rights reserved.

{third_party_notice_texts}

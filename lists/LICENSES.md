# Licences of the bundled lists

Close Call's code is GPL-3.0-or-later. The domain lists in this directory keep their upstream
licences. Every file here is a normalised snapshot of one source: comments, hosts-file addresses
and invalid names removed, names lowercased and converted to punycode, duplicates dropped, lines
sorted. Nothing else is changed. `manifest.toml` records the source URL, licence, snapshot date,
SHA-256 and entry count of each file. Licences were checked against the upstream repositories on
2026-11-16.

The app embeds these lists in its binary, so the binary is distributed under the terms of the GNU
GPL version 3. Every licence below allows that.

| List | Source | Licence | Licence checked in |
|---|---|---|---|
| `hostsvn-adult` | [bigdargon/hostsVN](https://github.com/bigdargon/hostsVN), adult extension, via StevenBlack/hosts | MIT | upstream `LICENSE` |
| `pornhosts` | [Clefspeare13/pornhosts](https://github.com/Clefspeare13/pornhosts), via StevenBlack/hosts | MIT | header of the list file (see below) |
| `sinfonietta-porn` | [Sinfonietta/hostfiles](https://github.com/Sinfonietta/hostfiles) `pornography-hosts`, via StevenBlack/hosts | MIT | upstream `LICENSE` |
| `sinfonietta-snuff` | [Sinfonietta/hostfiles](https://github.com/Sinfonietta/hostfiles) `snuff-hosts`, via StevenBlack/hosts | MIT | upstream `LICENSE` |
| `tiuxo-porn` | [tiuxo/hosts](https://github.com/tiuxo/hosts) `porn`, via StevenBlack/hosts | CC-BY-4.0 | upstream `LICENSE` |
| `hagezi-nsfw` | [hagezi/dns-blocklists](https://github.com/hagezi/dns-blocklists) `wildcard/nsfw-onlydomains.txt` | GPL-3.0 | upstream `LICENSE` and README |
| `ut1-bank`, `ut1-webmail`, `ut1-financial`, `ut1-dating`, `ut1-gambling` | [UT1 blacklists](https://dsi.ut-capitole.fr/blacklists/index_en.php), Université Toulouse Capitole, via the [olbat/ut1-blacklists](https://github.com/olbat/ut1-blacklists) mirror | CC-BY-SA-4.0 | mirror README and `blacklists/cc-by-sa-4-0.pdf` |
| `seed-banking`, `seed-webmail`, `seed-medical` | `lists/seeds/*.txt` in this repository | GPL-3.0-or-later | – |

## Notes per source

**StevenBlack/hosts extensions.** [StevenBlack/hosts](https://github.com/StevenBlack/hosts)
(MIT, Copyright © 2026 Steven Black) keeps a copy of each third-party porn list under
`extensions/porn/`, and we download those copies. Its `alternates/porn-only` file merges the same
sources plus one we leave out (see below), so we take the sources one by one instead.

**pornhosts.** The upstream repository was blocked by GitHub on 2026-11-16, so its `LICENSE`
file could not be read. The list file itself states "License: MIT" and links that file, and
StevenBlack/hosts records MIT for it as well.

**HaGeZi.** GPL-3.0 requires the licence text to travel with every copy. The repository's
`LICENSE` file is that text, and the app's licences screen must show it.

**UT1 (CC BY-SA 4.0).** Attribution: UT1 blacklists by Fabrice Prigent, Université Toulouse
Capitole, licensed under [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/);
changed as described at the top of this file. The official site answered 503 (maintenance) on
2026-11-16, so the licence was read from the daily mirror, which ships the licence text.

Compatibility: Creative Commons lists GPLv3 as a
[compatible licence](https://creativecommons.org/share-your-work/licensing-considerations/compatible-licenses/)
for BY-SA 4.0 adaptations (declared on 8 October 2015), and the FSF's licence list says the
same. The compatibility is one-way: BY-SA 4.0 material may go into a GPLv3 work, not the
reverse. It names GPL version 3 only, which is why the binary is distributed under GPLv3 terms
even though our own code also allows later versions. The `.txt` files themselves stay under
CC BY-SA 4.0.

**tiuxo (CC BY 4.0).** Attribution: list by tiuxo, licensed under
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/), changed as described above. The FSF
lists CC BY 4.0 as compatible with the GNU GPL.

## MIT notices

The MIT licence requires its notice in all copies:

- hostsVN: Copyright (c) 2026 BigDargon
- Sinfonietta/hostfiles: Copyright (c) 2016 Sinfonietta
- pornhosts: Copyright (c) Clefspeare13 and contributors
- StevenBlack/hosts: Copyright © 2026 Steven Black

> Permission is hereby granted, free of charge, to any person obtaining a copy of this software
> and associated documentation files (the "Software"), to deal in the Software without
> restriction, including without limitation the rights to use, copy, modify, merge, publish,
> distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the
> Software is furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all copies or
> substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING
> BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
> NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
> DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

## Left out

- **brijrajparmar27/host-sources** (part of StevenBlack's porn extension, 233 entries).
  StevenBlack/hosts records it as CC BY 4.0, but the upstream repository has no licence file and
  no README, so there is no grant from the author we could rely on. Without one the default is
  all rights reserved. HaGeZi NSFW and the other sources cover the same kind of sites.
- **UT1 `adult`.** About 4.6 million entries, far over the size budget.

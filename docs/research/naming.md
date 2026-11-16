# Név: Close Call

**Javaslat: Close Call.** Repo és CLI: `closecall`. A magyar felületen alcímként megmaradhat a „Bezártam?”.

## Miért ez
- **Három jelentés egy névben.** *Close*: ezt csinálja az app. *Call*: a Zoom/Teams/Meet hívás, ami a fő trigger. *Close call*: „egy hajszálon múlt”, vagyis az a pillanat, amikor majdnem kivetítetted a nyitott fület.
- **Két alapszó.** Angolul azonnal érthető, magyar diákok is ismerik. 9 karakter, nincs benne ékezet vagy kötőjel, CLI-parancsnak is kényelmes (`closecall now`).
- **Tiszta terep.** A crates.io, PyPI, Homebrew formula és cask szabad, a `close-call` npm-en és crates.io-n is szabad. Képernyőmegosztás vagy adatvédelem területén nem találtam ilyen nevű terméket.

## Mire figyelj
- npm-en a `closecall` foglalt (egy 2024-es, egyverziós LLM-segédcsomag). Ha valaha kell npm csomag: `close-call` vagy scoped név.
- A `CloseCall` GitHub-felhasználónév foglalt (2015 óta inaktív). A repó mehet a saját fiókod alá `closecall` néven. Ha szervezet kell, a `closecall-app` szabad.
- Domainek: a closecall.app parkolt és eladó (1999 USD), a closecall.dev egy pentester oldala, a closecallapp.com parkolt. A closecall.io és a getclosecall.com DNS szerint nincs regisztrálva (whois-szal nem ellenőriztem).
- **Védjegyet nem néztem** (USPTO, EUIPO, HIPO). A „close call” gyakori kifejezés filmek és játékok címében. Nyilvános kiadás előtt érdemes egy EUIPO eSearch-et futtatni a 9-es osztályban.
- Elsőre „hívásbontó” appnak is olvasható. A README első mondata ezt rendbe teszi.

## Második helyezettek
| Név | Slug | Pont | Röviden |
|---|---|---|---|
| Coast Clear | `coastclear` | 7,5 | Jó idióma, a regiszterek szabadok. Van viszont CoastClear ingatlanos webtermék (coastclear.app), egy iOS cápaészlelő app és egy kis macOS menüsoros eszköz ugyanezzel a névvel. |
| Did I Close? | `didiclose` | 7 | A legtisztább: sehol semmi ütközés, a domainek is szabadnak tűnnek. Gyengébb márkanév, és „didi close”-nak is olvasható. |
| Are You Decent? | `areyoudecent` | 6,5 | Vicces és tiszta, de 12 karakter, CLI-nek hosszú. |

## A saját ötleteid
- **Bezártam**: mindenhol szabad, de angolul kimondhatatlan. Magyar alcímnek való.
- **Elfelejtettem**: 13 betű, angolul nem működik.
- **Did I Forget**: emlékeztető appnak hangzik, és már van néhány kis repó ilyen néven.
- **Hmm did I close it**: slugnak túl hosszú. A „Did I Close?” ennek a rövid formája, és a 3. helyen áll.

## Versenytársak (a névkeresés közben kerültek elő)
Ezek a nevek foglaltak, de pozicionálás szempontjából is fontosak:
- **CleanSlate** (macOS, Product Hunt, 2024): képernyőmegosztás indulásakor elrejti a Chrome-tabok címét és faviconját, a Dockot és az asztali ikonokat.
- **Hush** (poteam.pro, Mac App Store): Zoom/Meet/Teams megosztásnál automatikusan elrejti a privát ablakokat.
- **Stagehand: Screenshare Curtain**, **Desktop Curtain** (Many Tricks), **Display Curtain**: Mac App Store.
- **Cloakly** (Windows), **WindowCloak** (macOS, nyílt forrású), **Invisiwind** (Windows), **Unshown** (macOS, fejlesztés alatt).
- **NothingHere** (macOS, GPL-3.0): gyorsbillentyűre elrejti az ablakokat, és megnyit egy takaródokumentumot.
- **ScreenShy** (AI meeting-asszisztens), **NoPeek – Screen Privacy** (Mac App Store).
- **leftopen** (macOS menüsor-app és CLI): nyitva hagyott localhost-folyamatokat zár be.
- Böngészőbővítmények: Panic Tabs, HideMyTabs, DiscreetTab, TabShield, Hide Tab Info, Stealth Browser Assistant (Firefox).

A talált leírások alapján ezek ablakokat rejtenek el vagy tabcímeket takarnak ki. Egyik sem írja magáról, hogy több platformon fut, hogy a tabokat és dokumentumokat szabályok szerint bezárja, vagy hogy kezeli a mentetlen fájlokat. Ez a lista nem teljes.

## Módszer és korlátok
- Mind a 45 brainstormolt névnél ellenőriztem pontos névre: npm, crates.io, PyPI, Homebrew (formula és cask), GitHub-repókeresés csillag szerint, Mac App Store (iTunes Search API, macSoftware). A legígéretesebbeknél webkeresés is volt (Chrome Web Store, Microsoft Store, cégek), és a domaineket DNS-sel néztem meg. Dátum: 2026-11-16.
- Futás közben **elfogyott a webkeresési keret**. A Zárva, ClearDeck, Safe to Share, Unblush, NoCringe, Ciki és Huss nevekhez emiatt csak regiszter-, GitHub- és Mac App Store-ellenőrzés készült, Chrome Web Store, Microsoft Store és védjegy nem. Ha kell, egy következő üzenettel folytatható.
- A végleges listából kihagytam, de ellenőriztem: decent, bashful, demure, chaperone, decorum (a `tauri-plugin-decorum` miatt Tauri-appnál zavaró lenne), modesty, curtaincall, prescreen, preshare, clearstage, showready, closey, facepalm, headsup, whoopsie (Ubuntu hibajelentő), oopsie. Mindegyik foglalt vagy gyenge.
- A regiszterek gyorsan változnak: az `allclear` npm-et 2026-11-01-én, a `nopeek` npm-et 2026-11-02-án foglalták le. A repó létrehozása előtt érdemes újra ellenőrizni a nevet.
- Fájlt nem írtam, a projekt helye (developer/personal mappa) a scaffolding lépés dolga.
# Kvoteven

**Dit AI-budget har fået et kæledyr.**

Et lille pixeldyr i proceslinjen (menulinjen på Mac) viser din resterende **Codex-ugekvote**
eller **DeepSeek-API-saldo**. Masser tilbage? Det er glad. Ved at løbe tør? Det bliver søvnigt.
Du skal ikke åbne en hjemmeside for at tjekke ét tal.

[English](README.md) · [Installation](docs/setup.md) · [Sikkerhed](SECURITY.md) ·
[Releases](https://github.com/Mbvjdev/Kvoteven/releases)

> **Offentlig beta — v0.4.0-beta.1.** [Hent og test](https://github.com/Mbvjdev/Kvoteven/releases/tag/v0.4.0-beta.1).
> Mac-DMG’en til Apple Silicon har bestået lokale native tests. Windows, Linux og
> universal-Mac testes i CI; pakker tilføjes, efterhånden som de består.
> Ikke produktionsgodkendt eller distributionssigneret. Se [kendte fejl](docs/beta-0.4.0.md).

![Kvoteven desktop: DeepSeek på engelsk og Codex på dansk, begge med tydeligt markerede demotal](docs/assets/desktop-overview.png)

*Desktopbrugerfladen i eksplicit offline-demo. Alle tal er fiktive, aldrig
vedligeholderens konto. Denne UI-forhåndsvisning dokumenterer ikke en native pakketest.*

## Hvad får du?

- Codex: resterende ugekvote og næste reset.
- DeepSeek: officiel kontosaldo og nettoændring siden første måling i appkørslen.
- Valgbart **System / English / Dansk**. Tekster, tal og datoer følger sproget.
- Lyst/mørkt udseende.
- Opdatering hvert femte minut for den valgte udbyder. Gamle/fejlramte tal bliver `—`.
- Ingen Kvoteven-server, analytics, modelkald, kreditkøb eller automatisk optankning.

## Understøttede platforme (0.4.0-kandidat)

| Platform | Pakke | Målgruppe | Signering i dag |
|---|---|---|---|
| macOS | Universal `.dmg` (Apple Silicon + Intel) | macOS 13+ | Ad-hoc — **ikke** Developer ID, **ikke** notariseret |
| Linux | `.deb` (x86_64) | Ubuntu 22.04+ | Usigneret; SHA-256-kontrolsummer udgives |
| Windows | NSIS-installationsprogram (x64, pr. bruger) | Windows 10 / 11 + WebView2 | Usigneret; SmartScreen kan advare |

Det er **byggemål**, ikke testede udgivelser: ingen platformspakke har bestået en CI-kørsel
endnu. Se [Distribution og signering](docs/distribution.md) for præcis hvad der er og ikke er
verificeret.

## Installer

### Download en offentlig beta

Tilgængelige pakker ligger under
[GitHub Releases](https://github.com/Mbvjdev/Kvoteven/releases/tag/v0.4.0-beta.1) med SHA-256-kontrolsummer.
Manglende platforme er endnu ikke klar; byg eventuelt fra kilden. Forvent advarsler fra styresystemet, fordi pakkerne ikke er
distributionssigneret: Gatekeeper på macOS, SmartScreen på Windows og intet signeret
pakkelager på Linux.

### Byg fra kilden

Du skal bruge [Node.js 22 LTS](https://nodejs.org/) og [Rust](https://www.rust-lang.org/tools/install)
(1.98.x er det, CI fastlåser). Den færdige app kræver hverken Node, Python eller Rust i kørselstid.

```sh
git clone https://github.com/Mbvjdev/Kvoteven.git
cd Kvoteven/desktop
npm ci --ignore-scripts
npm test
npm run build
cargo test --manifest-path src-tauri/Cargo.toml --locked
npm run tauri -- build --bundles app,dmg   # macOS; se docs/setup.md for Linux/Windows
```

Linux kræver desuden Tauri's systembiblioteker (se [opsætning](docs/setup.md)).
Der er ingen autostart eller automatisk opdatering; du opdaterer manuelt og tjekker
kontrolsummer fra en udgivelsesside.

### Prøv dyret, før du forbinder noget

Appen har en eksplicit offline-demo. Åbn Indstillinger og slå **Demotilstand** til, eller start
den native binære fil med:

```sh
# fra desktop/src-tauri/target/release/ på macOS/Linux, .exe på Windows
./kvoteven --demo
```

Panelet siger **Demo · eksempeldata** og viser faste fiktive tal (42,50 USD / 68 %). Denne
tilstand læser aldrig adgangsoplysninger og kalder aldrig en udbyder. Den bruges aldrig som
erstatning for en mislykket live-aflæsning.

### Tilslut dine eksisterende konti

- **DeepSeek** kræver hverken Hermes, Node, Python eller Rust: indtast din API-nøgle i
  Indstillinger (et maskeret felt), så gemmes den i systemets nøglering — Keychain på macOS,
  Credential Manager på Windows, Secret Service på Linux. Der er ingen plaintext-fallback.
- **Codex** læser kvote via én af to skrivebeskyttede kilder, valgbar i Indstillinger
  (**Auto / Codex CLI / Hermes**):
  - **Codex CLI** — installér og log ind med den officielle CLI (`codex login` i en terminal;
    OS-versioner efter de [officielle docs](https://developers.openai.com/codex/)). Kvoteven
    udfører ikke selve login.
  - **Hermes** — genbrug din eksisterende **default**-profils skrivebeskyttede forbrugsoutput.
    Der kopieres ingen OAuth eller tokens ud af Hermes.

**Du behøver ikke begge udbydere.** Vælg den, du bruger; kun den valgte udbyder opdateres
løbende. Valg af udbyder, sprog og Codex-kilde huskes på tværs af starter.

## Nøglerne bliver hos dig

Kvoteven uploader ikke adgangsoplysninger til en projektserver. Der er ingen projektserver.

DeepSeek-nøglen ligger kun i systemets nøglering og sendes i en authorization-header til
`https://api.deepseek.com/user/balance`. Den kommer ikke i procesargumenter, appindstillinger,
skærmbilleder eller repository'et. Codex læses gennem den officielle CLI's `app-server`-protokol
eller Hermes' eksisterende forbrugskommando; Kvoteven gemmer ingen Codex-adgangsoplysninger.

Saldo og kvote holdes i hukommelsen. Kun udbyder-, sprog- og Codex-kildevalg gemmes — aldrig
nøgler, logins eller kvotedata. Live-verifikationskommandoer udskriver kontotal lokalt, så
**læg ikke deres output i offentlige issues**.

Se den præcise afgrænsning i [SECURITY.md](SECURITY.md).

## Det betyder tallene

**Codex er ikke et samlet loft for hele ChatGPT.** Visningen gælder den kvote, udbyderen
returnerer for Codex. Manglende ugedata erstattes ikke af et andet tidsvindue.

**DeepSeek-saldoændring er ikke en faktura.** Forbrug, optankning og bonus kan alle ændre
saldoen. Sammenligningen starter ved første aflæsning og nulstilles ved genstart.
USD og CNY holdes adskilt. Humørgrænserne er vejledende, ikke et løfte om et bestemt antal
tokens. Dyret stopper ikke andre apps i at bruge kontoen.

Hvis værktøjet sparer dig for dashboard-ture, så giv det en stjerne på GitHub.
Oversættelser, opsætningsrapporter fra andre styresystemer og verificerede testrapporter fra
rigtige maskiner er velkomne. Send aldrig nøgler, loginfiler eller billeder af en privat konto
med et issue.

## Den oprindelige macOS-app

Version 0.3.0 — den native SwiftUI-menulinje-app — er bevaret som en legacy-udgave under
`Sources/` og `scripts/build_app.py`. Den er ikke den primære distribution fremover; det er
Tauri-desktop-appen ovenfor. Bland ikke de to byggesystemer eller testsæt.

MIT-licens. Uafhængigt projekt, ikke officielt tilknyttet OpenAI, DeepSeek,
Nous Research eller Tamagotchi/Bandai.

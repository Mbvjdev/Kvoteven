# Kvoteven

**Dit AI-budget har fået et kæledyr.**

Et lille pixeldyr i Mac'ens menulinje viser din resterende **Codex-ugekvote** eller
**DeepSeek-API-saldo**. Masser tilbage? Det er glad. Ved at løbe tør? Det bliver søvnigt.
Du skal ikke åbne en hjemmeside for at tjekke ét tal.

[English](README.md) · [Installation](docs/setup.md) · [Sikkerhed](SECURITY.md)

![Kvoteven med tydeligt markerede demotal, ikke en rigtig konto](docs/assets/overview.png)

## Hvad får du?

- Codex: resterende ugekvote, næste reset og øvrige tidsvinduer fra udbyderen.
- DeepSeek: officiel kontosaldo og nettoændring siden første måling i appkørslen.
- Valgbart **System / English / Dansk**. Tekster, tal og datoer følger sproget.
- Native macOS-app, lyst/mørkt udseende og understøttelse af reduceret bevægelse.
- Opdatering hvert femte minut for den valgte udbyder. Gamle/fejlramte tal bliver `—`.

## Installer

Dette er en tidlig udviklerversion. Livekonti kræver en eksisterende
[Hermes Agent-installation](https://hermes-agent.nousresearch.com/docs/).
Du skal bruge macOS 13+, Swift 5.9+ og Python 3.

```sh
git clone https://github.com/Mbvjdev/Kvoteven.git
cd Kvoteven
swift test
python3 -m unittest discover -s scripts -p 'test_*.py' -v
python3 scripts/build_app.py --install
open ~/Applications/Kvoteven.app
```

Luk den gamle app, før du installerer en opdatering. Appen er ad-hoc-signeret, ikke
Apple-notariseret. Den installerer ingen automatisk opstart.

Prøv uden konti eller netværk:

```sh
.build/release/Kvoteven --demo --language da
```

Demo vises tydeligt i panelet og bruges aldrig som erstatning for en mislykket liveaflæsning.

## Nøglerne bliver hos dig

Konfigurér dine konti i Hermes, ikke i dette repo. Kvoteven genbruger Hermes' default-profil.
Codex-login håndteres af Hermes. DeepSeek-helperen sender kun den relevante nøgle til
DeepSeeks officielle saldo-endpoint; appen opretter ikke et ekstra nøglelager.

Der er ingen Kvoteven-server, analytics, modelkald, kreditkøb eller automatisk optankning.
Saldo og kvote holdes i hukommelsen. Kun udbyder- og sprogvalg gemmes.
Se den præcise afgrænsning i [SECURITY.md](SECURITY.md).

## Det betyder tallene

**Codex er ikke et samlet loft for hele ChatGPT.** Visningen gælder den kvote, udbyderen
returnerer for Codex. Manglende ugedata erstattes ikke af et andet tidsvindue.

**DeepSeek-saldoændring er ikke en faktura.** Forbrug, optankning og bonus kan alle ændre
saldoen. Sammenligningen starter ved første aflæsning og nulstilles ved genstart.
USD og CNY holdes adskilt. Humørgrænserne er 20 og 5 enheder i den viste valuta, ikke
et løfte om et bestemt antal tokens. Dyret stopper ikke andre apps i at bruge kontoen.

Hvis værktøjet sparer dig for dashboard-ture, så giv det en stjerne på GitHub.
Oversættelser og konkrete opsætningsrapporter er velkomne. Send aldrig nøgler,
loginfiler eller billeder af en privat konto med et issue.

MIT-licens. Uafhængigt projekt, ikke officielt tilknyttet OpenAI, DeepSeek,
Nous Research eller Tamagotchi/Bandai.

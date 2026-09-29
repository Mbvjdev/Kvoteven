# Launch copy

Drafts for sharing the project. These are not claims of existing traction, and no social
posts are sent automatically.

## Short post

I got tired of opening a dashboard just to check one number, so I made the number a pet.

Kvoteven lives in your Mac's menu bar. It shows your remaining Codex weekly quota or
DeepSeek API balance, and gets sleepy when you're running low.

Open source, English/Danish, no Kvoteven cloud. Early preview; Hermes required for live accounts.

https://github.com/Mbvjdev/Kvoteven

## Show HN title

Show HN: Kvoteven – a menu-bar pet for your AI quota and API balance

## Longer introduction

I wanted a quick answer to “how much have I got left?” without interrupting what I was
building. Kvoteven puts that answer next to a tiny pixel pet in the macOS menu bar.

It currently supports weekly Codex subscription quota and DeepSeek's official API balance.
Those are different things, so the UI treats them differently: there is no invented weekly
DeepSeek quota, and balance change is not presented as a billing report.

The app is native SwiftUI/AppKit. It uses an existing Hermes installation for credentials,
has no project backend or analytics, and offers an explicit offline demo. English and Danish
are selectable. This is an early preview, not a notarized App Store app.

I would love setup reports on other Macs and ideas for useful provider integrations.

## Dansk

Jeg blev træt af at åbne en hjemmeside bare for at tjekke ét tal. Så jeg gav tallet et kæledyr.

Kvoteven bor i Mac'ens menulinje og viser din Codex-ugekvote eller DeepSeek-API-saldo.
Når du er ved at løbe tør, bliver det lille dyr søvnigt.

Open source, valgbart dansk/engelsk og ingen Kvoteven-server. Tidlig version; Hermes kræves
til livekonti. Koden ligger her: https://github.com/Mbvjdev/Kvoteven

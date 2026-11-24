# DiaryCraft — piano di implementazione v1 (bozza)

Stato: v1 del 24/11/2026, scritta da Claude; passi 1-5 realizzati nella prima PR, tranne l’evidenziazione del Markdown. Decisioni di Mauro già prese (24/11/2026): nome DiaryCraft; l'app nasce dall'idea logbook (OneDrive\Progetti\03 Prototype\logbook), la scrive Claude direttamente come BalanceCraft e non con Craft Anything, e il metodo di Kimbro (mappa-pensieri) ne diventa una funzionalità.

## Punto di partenza

- `mmazzieri/LogBook` contiene solo lo scheletro di ABP Framework generato nel 2021 (identità, permessi, Blazor, test d'esempio). Non c'è nessuna entità del diario e il prototipo Windows Forms del 2014 non è nel repository. Non c'è codice da portare.
- La specifica è la nota del 16/12/2014: cartella radice configurabile; un file per giorno in sottocartelle per anno, mese e giorno; finestra con cartella e ricerca in alto, calendario ed editor sotto; salvataggio automatico al cambio di data e alla chiusura; allegati facoltativi (gli altri file nella cartella del giorno); le versioni non desktop salvano su OneDrive o simili.
- `mmazzieri/complete-thought-map` contiene il libro di Kimbro (CC BY-SA 1.0) e il README di un programma «ibrido» mai scritto. Il software lì è dichiarato GPL-3.0, ma non c'è codice: la parte Kimbro di DiaryCraft si scrive da zero sotto MIT OR Apache-2.0.

## Obiettivo

App desktop locale in Rust ed egui per tenere un diario: una voce per giorno, scritta in testo semplice, salvata come file normale in una cartella che l'utente sceglie e che può stare in locale o in una cartella sincronizzata (OneDrive, Google Drive, Dropbox, Syncthing) o su un disco condiviso. I file restano leggibili e modificabili con qualunque editor anche senza l'app. Più avanti: candidatura come community app nell'ArtCraft Launcher.

Fuori dalla v1: account, server, cifratura, app mobile, build web, MCP/CLI, AI.

## Vincoli

- Le stesse regole di BalanceCraft e delle app Craft: `unsafe` vietato, clippy `-D warnings`, `unwrap`/`expect`/`panic` negati fuori dai test, dipendenze solo dall'allowlist delle licenze controllata con `cargo deny`, nessun codice GPL/LGPL/AGPL nemmeno letto.
- Clean room: nessun testo, icona o asset di altri programmi di diario; nessun nome di prodotto nell'interfaccia.
- **Il testo dell'utente non si perde mai.** Ogni scrittura è atomica (file temporaneo e rinomina); se il file sul disco è cambiato da quando l'app l'ha letto, l'app non lo sovrascrive: tiene entrambe le versioni e lo dice. È la regola che conta più delle funzioni.

## Architettura (workspace Cargo, come BalanceCraft)

```
crates/
  model/     tipi: Entry (data, testo, metadati facoltativi), DiaryConfig, Subject, SpeedThought…
  engine/    operazioni pure: apertura del giorno, ricerca, calendario (giorni con voce),
             parsing e scrittura del front matter, parte Kimbro; orologio passato come parametro
  file/      layout della cartella, lettura e scrittura atomica, controllo delle modifiche esterne,
             copie di conflitto dei client di sincronizzazione, allegati
  ui-egui/   interfaccia
apps/
  diarycraft/   eseguibile desktop (eframe)
xtask/          ci, packaging
```

`model` ed `engine` non fanno I/O e compilano in wasm32, per una futura build web.

## Formato su disco

- Cartella radice scelta dall'utente. Una voce: `AAAA/MM/AAAA-MM-GG.md`, Markdown in UTF-8. Il nome con la data intera rende il file riconoscibile anche fuori dalla sua cartella.
- Il file è solo testo. Metadati facoltativi (luogo, umore, etichette) in un front matter YAML minimo in testa al file, che l'app conserva anche quando contiene chiavi che non conosce.
- Allegati: cartella `AAAA/MM/AAAA-MM-GG/` accanto alla voce; l'app li elenca e li apre con il programma di sistema, e li inserisce nel testo come link relativi Markdown.
- Impostazioni del diario in `.diarycraft/config.json` nella radice (versione del formato, lingua del calendario, primo giorno della settimana), così sono uguali su ogni computer che apre la cartella. Le impostazioni della finestra restano sul computer.
- Import: la v1 legge anche i layout della nota del 2014 (`AAAA/MM/GG.txt` e `AAAA/MM/AAAAMMGG.txt`) e li mostra; la migrazione al layout nuovo è un comando esplicito, non automatico.

## Interfaccia v1

- In alto: la cartella aperta, la ricerca nel testo di tutte le voci, il pulsante Oggi.
- A sinistra il calendario del mese, con i giorni che hanno una voce evidenziati e le frecce per mese e anno; sotto, l'elenco delle voci recenti e i risultati della ricerca.
- A destra l'editor del giorno: testo Markdown con evidenziazione leggera (titoli, grassetto, corsivo, elenchi, link), contatore delle parole, allegati.
- Salvataggio automatico un secondo dopo l'ultima modifica, al cambio di giorno e alla chiusura. La voce di un giorno senza testo non crea file.
- Controllo periodico della cartella: se un'altra copia dell'app o il client di sincronizzazione cambia la voce aperta e l'editor non ha modifiche, l'app ricarica; se le ha, salva il testo locale come copia di conflitto (`AAAA-MM-GG (conflitto PC).md`) e mostra le due versioni affiancate per unirle a mano.
- Lingue: italiano e inglese. Tema chiaro e scuro.

## Seconda fase: il metodo di Kimbro

Si costruisce dopo la v1, sugli stessi file di testo, senza cambiare il formato delle voci.

- **Speed list**: cattura rapida di un pensiero da una scorciatoia globale o dal campo in cima all'app, con numero progressivo, *hint* di una-tre parole e soggetto facoltativo. I pensieri del giorno finiscono in una sezione della voce del giorno, così il diario resta l'unico posto dove si scrive.
- **Soggetti**: una cartella per soggetto con la pagina P&P (scopo e principi, inclusioni ed esclusioni), le note di approfondimento (POI), ricerca, fonti, progetti.
- **Smistamento**: dalla speed list del giorno ogni pensiero si sposta nel soggetto giusto, a mano; la proposta automatica con un LLM guidato dalle P&P è un passo successivo e facoltativo.
- **Mappe**: SMOC per soggetto e GSMOC su tutti i soggetti, generate dai collegamenti tra le note e modificabili.
- Il libro: Mauro aveva deciso (08/11/2026) che il repository ne contenga la copia integrale. Si può fare anche qui: cartella `book/` con i file originali non modificati, testo della licenza CC BY-SA 1.0 e riga in `ATTRIBUTION.md`; la documentazione che riprende il libro va sotto CC BY-SA 1.0, il codice sotto MIT OR Apache-2.0. Il nome di Kimbro non si usa in modo da far pensare a un suo avallo.

## Repository

Proposta: riusare `mmazzieri/LogBook`. Mauro lo rinomina in `DiaryCraft` dalle impostazioni di GitHub (i vecchi link reindirizzano). Claude lavora su un ramo che sostituisce lo scheletro ABP con il workspace Rust e apre una PR. La storia del 2021 resta nella cronologia. `complete-thought-map` resta com'è finché la seconda fase non lo assorbe; allora si archivia, oppure si cambia la licenza a MIT OR Apache-2.0 per riusarne il README.

## Passi

1. Workspace, CI (`cargo xtask ci`: fmt, clippy, test, deny) e app vuota che apre una cartella.
2. Formato su disco e crate `file`: lettura, scrittura atomica, elenco delle voci, test sui casi di conflitto.
3. Calendario, editor e salvataggio automatico.
4. Ricerca, allegati, import dei layout del 2014.
5. Modifiche esterne e copie di conflitto, provate con due istanze sulla stessa cartella sincronizzata.
6. Pacchetti per Windows, macOS e Linux come BalanceCraft; candidatura al launcher.
7. Seconda fase: Kimbro.

## Domande per Mauro

- Repository: va bene riusare LogBook rinominato, o Mauro preferisce un repository nuovo?
- Dove sta il diario che Mauro userà per provarlo (cartella locale o OneDrive), e se esistono voci vecchie da importare.

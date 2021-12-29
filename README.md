# Tantabus
Tantabus is a WIP hobby Chess and Chess960 engine.<br>
It is a "rewrite" of [Lunatic](https://github.com/analog-hors/lunatic).<br>
The code is restructured and a bit cleaner, and it also uses my own [`cozy-chess`](https://github.com/analog-hors/cozy-chess) library in place of [`chess`](https://github.com/jordanbray/chess).<br>
Play me on lichess: https://lichess.org/@/TantabusEngine.

## Features
### Movegen
- Fixed shift fancy black magic bitboards using [`cozy-chess`](https://github.com/analog-hors/cozy-chess)
### Search
- Principal variation search
- Aspiration windows
- Transposition table
    - "Always replace" replacement scheme
- Quiescence search
- Extensions
    - Check extensions
- Reductions
    - Late move reductions
- Pruning
    - Null move pruning
    - Futility pruning
    - Reverse futility pruning
    - Negative SEE moves pruned in QSearch
- Move ordering
    - Hash move
    - Static exchange evaluation
    - Killer moves
    - History heuristic
### Evaluation
- [Automatically tuned with currently private tuner on an Ethereal dataset](https://github.com/analog-hors/tantabus/commit/8c894ffeed7516b05be0a8e8db6f1d96aa83904c)
- Piece square tables
    - Dedicated passed pawn tables
- Mobility evaluation (simple pseudo-legal counting)
- Tapered/phased evaluation (using Fruit-like method)
### Time management
- Uses a fixed percentage of time left

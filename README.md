# Tantabus
Tantabus is a WIP hobby Chess and Chess960 engine.<br>
It is a "rewrite" of [Lunatic](https://github.com/analog-hors/lunatic).<br>
The code is restructured and a bit cleaner, and it also uses my own [`cozy-chess`](https://github.com/analog-hors/cozy-chess) library in place of [`chess`](https://github.com/jordanbray/chess).

## Features
### Movegen
- Fixed shift fancy black magic bitboards using [`cozy-chess`](https://github.com/analog-hors/cozy-chess)
### Search
- Principal variation search
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
    - Negative SEE moves pruned in QSearch
- Move ordering
    - Hash move
    - Static exchange evaluation
    - Killer moves
    - History heuristic
### Evaluation
- Tuned piece square tables
    - [Tuned with currently private tuner on Ethereal dataset](https://github.com/analog-hors/lunatic/commit/28b85304aa71e1561883cf53976496b3dbba8fd8)
- Tapered/phased evaluation (using Fruit-like method)
### Time management
- Uses a fixed percentage of time left

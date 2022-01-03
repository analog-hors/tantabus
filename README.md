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
- [Automatically tuned with currently private tuner on the `lichess-big3-resolved` dataset](https://drive.google.com/file/d/1GfrNuDfD9Le-ZKKLxTHu0i3z6fbTn8JJ/view?usp=sharing)
- King relative symmetric piece-square tables
    - Dedicated passed pawn tables
- Mobility evaluation (simple pseudo-legal counting)
- Tapered/phased evaluation (using Fruit-like method)
### Time management
- Uses a fixed percentage of time left

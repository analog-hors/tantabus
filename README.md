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
    - History reductions
- Pruning
    - Null move pruning
    - Futility pruning
    - Reverse futility pruning
    - Late move pruning
    - Negative SEE moves pruned in QSearch
- Move ordering
    - Hash move
    - Capture moves
        - Losing captures delayed to last
    - Static exchange evaluation
    - Killer moves
    - History heuristic
### Evaluation
- HCE
    - No longer exists, used to train NNUE
    - [Automatically tuned with currently private tuner on the `lichess-big3-resolved` dataset](https://drive.google.com/file/d/1GfrNuDfD9Le-ZKKLxTHu0i3z6fbTn8JJ/view?usp=sharing)
    - King relative symmetric piece-square tables
        - Dedicated passed pawn tables
    - Mobility evaluation (simple pseudo-legal counting)
    - Bishop pair bonus
    - Rook on open file bonus
    - Rook on semiopen file bonus
    - Basic king safety using "virtual queen mobility" and attacked squares around the king
    - Tapered/phased evaluation (using Fruit-like method)
- NNUE
    - Simple 768 -> 32x2 -> 1 network trained with a simple PyTorch trainer
    - Two halves ordered by side to move and side not to move
### Time management
- Uses a fixed percentage of time left
- Aborts if it is estimated that the next iteration cannot be completed in time.

## Thanks
A (potentially incomplete) list of very useful resources:
- [The CPW wiki](https://www.chessprogramming.org/) - Extremely useful site for chess programming
- [the NNUE document](https://github.com/glinscott/nnue-pytorch/blob/master/docs/nnue.md) - Extremely useful for NNUE

Many engines have been very useful resources in the development of Tantabus.<br>
A (potentially incomplete) list of citations is listed in the code, annotated with `// CITE` comments.<br>
A (potentially incomplete) list of special thanks in no particular order:
- [Pali (Black Marlin author)](https://github.com/dsekercioglu/blackmarlin), for assisting me with various things during the development of Tantabus on top of being like, cool and stuff.
- [Jay (Berserk author)](https://github.com/jhonnold/berserk) for having hosted the OpenBench instance that Tantabus developed on, as well as suggesting various improvements.
- [Andrew (OpenBench and Ethereal author)](https://github.com/AndyGrant/Ethereal) for OpenBench. OpenBench has been an immensely helpful tool for engine development. Ethereal is also a very influential engine.
- MinusKelvin ([Cold Clear](https://github.com/MinusKelvin/cold-clear) and [Frozenight](https://github.com/MinusKelvin/frozenight) author), for being like, really cool and stuff.
- Anyone who donated CPU time to instance that hosts Tantabus.
- Other people I probably forgot about.

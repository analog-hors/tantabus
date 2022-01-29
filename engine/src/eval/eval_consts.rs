use super::mob::Mobility;
use super::pst::{PstEvalSet, KingRelativePst, Pst};
use super::{Evaluator, EvalTerms};
use super::eval_set::PieceEvalSet;

pub const PIECE_VALUES: PieceEvalSet<i16> = PieceEvalSet {
    pawn: 100,
    knight: 320,
    bishop: 330,
    rook: 500,
    queen: 900,
    king: 0,
};

pub const EVALUATOR: Evaluator = Evaluator {
    midgame: EvalTerms {
        piece_tables: PstEvalSet {
            pawn: KingRelativePst([
                [
                    [   0,    0,    0,    0],
                    [  99,  113,  126,  138],
                    [  66,   86,  114,  116],
                    [  57,   84,   86,   95],
                    [  50,   79,   83,  100],
                    [  50,   76,   77,   85],
                    [  41,   66,   64,   63],
                    [   0,    0,    0,    0],
                ],
                [
                    [   0,    0,    0,    0],
                    [  86,   98,  125,  138],
                    [ 127,  159,  158,  128],
                    [ 108,  121,  116,  118],
                    [  99,  115,  110,  109],
                    [ 112,  133,  108,   98],
                    [  88,  133,  104,   81],
                    [   0,    0,    0,    0],
                ],
            ]),
            knight: KingRelativePst([
                [
                    [ 178,  259,  269,  295],
                    [ 308,  319,  350,  369],
                    [ 309,  348,  375,  383],
                    [ 319,  333,  358,  375],
                    [ 313,  333,  345,  344],
                    [ 304,  325,  332,  339],
                    [ 284,  299,  312,  328],
                    [ 250,  301,  290,  303],
                ],
                [
                    [ 212,  284,  282,  334],
                    [ 328,  332,  400,  350],
                    [ 341,  366,  411,  415],
                    [ 344,  342,  383,  363],
                    [ 328,  352,  354,  355],
                    [ 325,  349,  356,  350],
                    [ 310,  308,  323,  326],
                    [ 265,  303,  309,  310],
                ],
            ]),
            bishop: KingRelativePst([
                [
                    [ 310,  300,  277,  262],
                    [ 323,  349,  339,  312],
                    [ 331,  348,  351,  365],
                    [ 324,  334,  354,  367],
                    [ 335,  335,  340,  358],
                    [ 342,  358,  348,  346],
                    [ 344,  346,  351,  332],
                    [ 332,  352,  339,  328],
                ],
                [
                    [ 297,  310,  284,  277],
                    [ 317,  325,  339,  344],
                    [ 345,  364,  377,  360],
                    [ 324,  334,  354,  358],
                    [ 349,  342,  347,  357],
                    [ 366,  365,  364,  348],
                    [ 342,  366,  354,  345],
                    [ 342,  345,  322,  335],
                ],
            ]),
            rook: KingRelativePst([
                [
                    [ 505,  508,  501,  497],
                    [ 490,  489,  506,  526],
                    [ 472,  494,  486,  493],
                    [ 466,  475,  476,  483],
                    [ 459,  460,  465,  470],
                    [ 463,  471,  470,  473],
                    [ 465,  470,  481,  482],
                    [ 488,  487,  488,  492],
                ],
                [
                    [ 547,  500,  520,  521],
                    [ 564,  547,  551,  513],
                    [ 536,  560,  529,  517],
                    [ 504,  498,  502,  484],
                    [ 491,  507,  483,  478],
                    [ 515,  532,  500,  485],
                    [ 486,  514,  494,  488],
                    [ 488,  493,  495,  498],
                ],
            ]),
            queen: KingRelativePst([
                [
                    [ 861,  868,  888,  916],
                    [ 894,  880,  884,  867],
                    [ 900,  896,  891,  896],
                    [ 888,  887,  889,  886],
                    [ 898,  894,  891,  891],
                    [ 908,  909,  901,  897],
                    [ 904,  902,  906,  907],
                    [ 892,  893,  902,  906],
                ],
                [
                    [ 905,  942,  920,  907],
                    [ 932,  890,  910,  873],
                    [ 928,  940,  915,  907],
                    [ 902,  894,  894,  885],
                    [ 914,  909,  900,  895],
                    [ 922,  925,  918,  898],
                    [ 900,  915,  912,  908],
                    [ 888,  883,  883,  902],
                ],
            ]),
            king: Pst([
                [ -10,    7,   13,   -8,   -3,    5,    9,   -2],
                [ -19,   13,    1,   38,   14,   18,   27,    8],
                [ -28,   46,   14,   -2,    9,   54,   40,    9],
                [ -26,  -13,  -18,  -63,  -74,  -33,  -30,  -69],
                [ -35,  -14,  -46,  -93,  -90,  -48,  -43,  -93],
                [   2,   35,  -20,  -43,  -37,  -21,   19,  -15],
                [  61,   58,   30,    7,    4,   21,   62,   50],
                [  53,   74,   52,  -23,   37,    3,   66,   64],
            ]),
        },
        mobility: Mobility {
            pawn: [-14, -7, 3, 28, 0],
            knight: [-63, -48, -41, -37, -34, -35, -35, -34, -33],
            bishop: [-68, -59, -52, -48, -40, -31, -27, -23, -23, -22, -19, -16, -12, 4],
            rook: [-128, -118, -116, -111, -114, -109, -107, -101, -100, -101, -99, -97, -97, -89, -91],
            queen: [-38, -41, -51, -50, -48, -45, -43, -45, -43, -41, -40, -40, -41, -39, -39, -38, -36, -38, -36, -35, -24, -22, -17, -14, 11, 31, 4, 10],
            king: [7, -11, -11, -18, -25, -35, -32, -43, -49],
        },
        passed_pawns: KingRelativePst([
            [
                [   0,    0,    0,    0],
                [   6,   25,   26,   34],
                [  22,   42,   33,   17],
                [  28,   22,   28,   15],
                [  18,    3,  -10,   -7],
                [   6,  -14,  -15,  -16],
                [   0,   -4,   -9,  -13],
                [   0,    0,    0,    0],
            ],
            [
                [   0,    0,    0,    0],
                [ -34,  -11,   16,   33],
                [ -66,  -41,    4,   13],
                [ -40,  -17,    7,   -2],
                [ -36,  -15,  -18,  -17],
                [ -15,   -5,  -28,  -19],
                [ -23,    8,   -6,   -2],
                [   0,    0,    0,    0],
            ],
        ]),
        bishop_pair: 24,
        rook_on_open_file: 38,
        rook_on_semiopen_file: 13,
        virtual_queen_mobility: [5, 37, 27, 25, 23, 23, 23, 20, 15, 9, 4, -2, -8, -17, -22, -26, -27, -25, -23, -15, -11, -11, -9, -4, -5, -4, -2, -1],
    },
    endgame: EvalTerms {
        piece_tables: PstEvalSet {
            pawn: KingRelativePst([
                [
                    [   0,    0,    0,    0],
                    [ 190,  180,  173,  164],
                    [ 100,  107,   84,  101],
                    [  97,   99,   86,   80],
                    [  85,   95,   82,   84],
                    [  83,   93,   80,   86],
                    [  85,   91,   82,   88],
                    [   0,    0,    0,    0],
                ],
                [
                    [   0,    0,    0,    0],
                    [ 221,  206,  198,  170],
                    [  92,  104,   73,   77],
                    [  79,   88,   74,   73],
                    [  71,   85,   76,   80],
                    [  65,   80,   78,   87],
                    [  62,   70,   79,   97],
                    [   0,    0,    0,    0],
                ],
            ]),
            knight: KingRelativePst([
                [
                    [ 248,  296,  310,  312],
                    [ 303,  320,  319,  313],
                    [ 309,  315,  335,  339],
                    [ 320,  329,  347,  351],
                    [ 323,  327,  348,  347],
                    [ 313,  324,  335,  339],
                    [ 297,  304,  317,  317],
                    [ 282,  287,  301,  305],
                ],
                [
                    [ 216,  266,  287,  299],
                    [ 283,  304,  295,  312],
                    [ 292,  306,  314,  325],
                    [ 307,  327,  338,  347],
                    [ 316,  324,  339,  347],
                    [ 313,  324,  334,  335],
                    [ 308,  310,  313,  318],
                    [ 295,  294,  304,  307],
                ],
            ]),
            bishop: KingRelativePst([
                [
                    [ 324,  324,  321,  330],
                    [ 311,  312,  314,  322],
                    [ 319,  315,  319,  309],
                    [ 319,  324,  318,  331],
                    [ 319,  324,  333,  324],
                    [ 324,  330,  328,  324],
                    [ 310,  310,  302,  318],
                    [ 303,  316,  310,  314],
                ],
                [
                    [ 306,  321,  321,  323],
                    [ 306,  321,  312,  306],
                    [ 317,  313,  316,  319],
                    [ 316,  316,  322,  325],
                    [ 310,  323,  324,  324],
                    [ 322,  326,  328,  325],
                    [ 314,  315,  312,  315],
                    [ 296,  310,  326,  317],
                ],
            ]),
            rook: KingRelativePst([
                [
                    [ 514,  511,  520,  520],
                    [ 516,  521,  521,  511],
                    [ 512,  507,  511,  505],
                    [ 513,  507,  510,  506],
                    [ 511,  508,  507,  503],
                    [ 502,  497,  498,  499],
                    [ 496,  498,  495,  494],
                    [ 502,  494,  499,  492],
                ],
                [
                    [ 498,  518,  516,  507],
                    [ 483,  500,  502,  510],
                    [ 480,  484,  486,  494],
                    [ 490,  495,  494,  497],
                    [ 494,  494,  498,  498],
                    [ 482,  479,  488,  489],
                    [ 483,  474,  487,  490],
                    [ 484,  496,  498,  489],
                ],
            ]),
            queen: KingRelativePst([
                [
                    [ 932,  925,  937,  927],
                    [ 905,  916,  936,  972],
                    [ 892,  894,  926,  947],
                    [ 904,  908,  919,  942],
                    [ 904,  910,  913,  930],
                    [ 888,  901,  910,  903],
                    [ 876,  890,  885,  887],
                    [ 886,  885,  876,  893],
                ],
                [
                    [ 895,  890,  938,  943],
                    [ 915,  939,  954,  982],
                    [ 918,  908,  949,  951],
                    [ 935,  943,  953,  953],
                    [ 925,  935,  931,  933],
                    [ 903,  922,  918,  912],
                    [ 867,  852,  873,  892],
                    [ 866,  869,  888,  876],
                ],
            ]),
            king: Pst([
                [ -83,  -47,  -30,   -8,  -24,  -16,  -15,  -82],
                [ -23,   15,   20,   21,   32,   39,   27,  -11],
                [ -11,   16,   37,   50,   53,   42,   29,   -5],
                [ -15,   18,   43,   62,   64,   48,   25,    0],
                [ -22,    7,   39,   60,   60,   37,   16,    3],
                [ -29,   -5,   21,   34,   33,   22,   -2,   -9],
                [ -47,  -27,   -7,    2,    5,   -6,  -30,  -43],
                [ -73,  -60,  -37,  -20,  -42,  -25,  -54,  -72],
            ]),
        },
        mobility: Mobility {
            pawn: [9, 29, 34, 21, 50],
            knight: [34, 30, 36, 31, 36, 41, 42, 41, 36],
            bishop: [-15, 1, 8, 18, 32, 45, 49, 55, 62, 62, 59, 59, 65, 53],
            rook: [102, 118, 121, 124, 130, 134, 134, 137, 145, 149, 150, 153, 155, 154, 153],
            queen: [7, 46, 152, 187, 197, 202, 212, 235, 243, 248, 257, 265, 272, 279, 284, 290, 294, 305, 311, 312, 310, 313, 311, 316, 296, 301, 302, 304],
            king: [0, -34, -24, -6, -3, 3, 3, 12, 11],
        },
        passed_pawns: KingRelativePst([
            [
                [   0,    0,    0,    0],
                [  97,   92,   74,   61],
                [ 133,  118,  101,   65],
                [  64,   53,   36,   38],
                [  34,   19,   17,   12],
                [   4,   11,    6,   -6],
                [   2,    7,    1,  -15],
                [   0,    0,    0,    0],
            ],
            [
                [   0,    0,    0,    0],
                [ 101,   98,   88,   65],
                [ 170,  149,  132,   93],
                [  94,   85,   67,   47],
                [  54,   49,   36,   24],
                [  12,   13,   10,    5],
                [  11,    0,   -2,   -7],
                [   0,    0,    0,    0],
            ],
        ]),
        bishop_pair: 63,
        rook_on_open_file: 6,
        rook_on_semiopen_file: 5,
        virtual_queen_mobility: [1, 19, 20, 5, 7, 3, 2, 4, 6, 7, 9, 12, 13, 14, 12, 13, 7, 5, 1, -6, -10, -14, -19, -25, -27, -30, -16, -11],
    },
};

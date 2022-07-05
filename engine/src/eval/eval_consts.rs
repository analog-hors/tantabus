use super::mob::Mobility;
use super::phased_eval::PhasedEval;
use super::pst::{PstEvalSet, KingRelativePst, Pst};
use super::{EvalWeights, EvalTerms};
use super::eval_set::PieceEvalSet;

pub const PIECE_VALUES: PieceEvalSet<i16> = PieceEvalSet {
    pawn: 100,
    knight: 320,
    bishop: 330,
    rook: 500,
    queen: 900,
    king: 0,
};

const fn e(mg: i16, eg: i16) -> PhasedEval {
    PhasedEval(mg, eg)
}

// CITE: These constants were tuned using various datasets.
// It was first tuned on a Zurichess dataset, then
// an Ethereal dataset, then a dataset called
// lichess-big3-resolved linked to me by Jay (Berserk author).
// Zurichess set: quiet-labeled.v7.epd from https://bitbucket.org/zurichess/tuner/downloads/
// Ethereal set: E12.33-1M-D12-Resolved from https://talkchess.com/forum3/viewtopic.php?t=75350
// lichess-big3-resolved: https://drive.google.com/file/d/1GfrNuDfD9Le-ZKKLxTHu0i3z6fbTn8JJ/view?usp=sharing
// The tuning code is currently private because it is cringe.
pub const EVAL_WEIGHTS: EvalWeights = EvalTerms {
    piece_tables: PstEvalSet {
        pawn: KingRelativePst([
            [
                [e(   0,    0), e(   0,    0), e(   0,    0), e(   0,    0)],
                [e( 103,  189), e( 115,  179), e( 127,  172), e( 141,  166)],
                [e(  71,  101), e(  88,  108), e( 115,   87), e( 115,  102)],
                [e(  60,   94), e(  84,   96), e(  88,   85), e(  96,   78)],
                [e(  54,   83), e(  82,   92), e(  85,   82), e(  99,   83)],
                [e(  55,   83), e(  76,   91), e(  79,   81), e(  86,   86)],
                [e(  44,   82), e(  68,   90), e(  67,   81), e(  64,   87)],
                [e(   0,    0), e(   0,    0), e(   0,    0), e(   0,    0)],
            ],
            [
                [e(   0,    0), e(   0,    0), e(   0,    0), e(   0,    0)],
                [e(  86,  222), e(  96,  204), e( 126,  199), e( 139,  172)],
                [e( 122,   94), e( 151,  107), e( 156,   75), e( 126,   78)],
                [e( 102,   80), e( 115,   90), e( 115,   75), e( 115,   73)],
                [e(  93,   71), e( 110,   84), e( 106,   75), e( 109,   80)],
                [e( 108,   66), e( 129,   79), e( 105,   78), e(  97,   87)],
                [e(  86,   61), e( 131,   71), e( 104,   77), e(  81,   95)],
                [e(   0,    0), e(   0,    0), e(   0,    0), e(   0,    0)],
            ],
        ]),
        knight: KingRelativePst([
            [
                [e( 178,  250), e( 255,  297), e( 267,  312), e( 294,  315)],
                [e( 309,  302), e( 322,  319), e( 351,  317), e( 369,  315)],
                [e( 308,  308), e( 347,  314), e( 376,  334), e( 380,  343)],
                [e( 320,  318), e( 332,  329), e( 357,  347), e( 374,  353)],
                [e( 315,  322), e( 334,  327), e( 347,  346), e( 346,  345)],
                [e( 307,  311), e( 327,  321), e( 335,  332), e( 342,  337)],
                [e( 285,  296), e( 301,  304), e( 314,  314), e( 328,  314)],
                [e( 250,  282), e( 300,  285), e( 288,  299), e( 303,  301)],
            ],
            [
                [e( 212,  217), e( 283,  266), e( 283,  289), e( 336,  300)],
                [e( 327,  284), e( 334,  303), e( 397,  298), e( 353,  312)],
                [e( 334,  291), e( 358,  307), e( 401,  314), e( 405,  331)],
                [e( 341,  309), e( 329,  332), e( 380,  341), e( 356,  353)],
                [e( 328,  316), e( 352,  323), e( 355,  339), e( 355,  347)],
                [e( 326,  311), e( 349,  321), e( 355,  330), e( 352,  333)],
                [e( 312,  304), e( 307,  308), e( 326,  311), e( 326,  317)],
                [e( 269,  294), e( 304,  294), e( 312,  299), e( 311,  304)],
            ],
        ]),
        bishop: KingRelativePst([
            [
                [e( 313,  323), e( 300,  323), e( 275,  322), e( 262,  330)],
                [e( 324,  308), e( 353,  310), e( 341,  313), e( 312,  322)],
                [e( 329,  318), e( 346,  313), e( 350,  320), e( 362,  311)],
                [e( 323,  318), e( 335,  322), e( 349,  321), e( 364,  334)],
                [e( 336,  316), e( 334,  325), e( 339,  331), e( 356,  325)],
                [e( 341,  326), e( 356,  330), e( 347,  325), e( 346,  323)],
                [e( 343,  311), e( 347,  307), e( 351,  301), e( 334,  315)],
                [e( 331,  302), e( 352,  315), e( 338,  309), e( 327,  311)],
            ],
            [
                [e( 297,  306), e( 308,  320), e( 283,  322), e( 276,  324)],
                [e( 313,  306), e( 325,  320), e( 338,  312), e( 346,  306)],
                [e( 332,  322), e( 358,  314), e( 370,  317), e( 356,  322)],
                [e( 321,  316), e( 335,  315), e( 352,  324), e( 353,  328)],
                [e( 348,  310), e( 340,  323), e( 347,  322), e( 355,  326)],
                [e( 365,  320), e( 364,  323), e( 363,  326), e( 347,  321)],
                [e( 342,  316), e( 367,  313), e( 355,  309), e( 345,  314)],
                [e( 345,  293), e( 345,  308), e( 326,  323), e( 333,  314)],
            ],
        ]),
        rook: KingRelativePst([
            [
                [e( 495,  517), e( 503,  512), e( 495,  521), e( 489,  522)],
                [e( 484,  520), e( 483,  523), e( 500,  525), e( 518,  516)],
                [e( 469,  512), e( 493,  506), e( 488,  510), e( 491,  505)],
                [e( 468,  511), e( 479,  503), e( 478,  508), e( 486,  504)],
                [e( 461,  508), e( 461,  507), e( 467,  505), e( 472,  501)],
                [e( 464,  501), e( 472,  496), e( 471,  497), e( 472,  497)],
                [e( 463,  495), e( 471,  496), e( 480,  496), e( 481,  493)],
                [e( 486,  499), e( 485,  493), e( 486,  497), e( 492,  491)],
            ],
            [
                [e( 541,  497), e( 498,  518), e( 517,  516), e( 514,  507)],
                [e( 554,  485), e( 543,  501), e( 540,  504), e( 508,  513)],
                [e( 529,  479), e( 557,  484), e( 527,  486), e( 517,  495)],
                [e( 503,  488), e( 497,  494), e( 503,  493), e( 487,  495)],
                [e( 488,  493), e( 505,  493), e( 480,  498), e( 481,  496)],
                [e( 513,  480), e( 528,  478), e( 496,  487), e( 484,  488)],
                [e( 486,  479), e( 512,  472), e( 493,  486), e( 488,  489)],
                [e( 488,  482), e( 489,  495), e( 496,  495), e( 497,  486)],
            ],
        ]),
        queen: KingRelativePst([
            [
                [e( 854,  932), e( 865,  925), e( 883,  936), e( 912,  924)],
                [e( 892,  906), e( 881,  913), e( 884,  935), e( 863,  973)],
                [e( 894,  895), e( 893,  895), e( 892,  923), e( 892,  948)],
                [e( 888,  903), e( 888,  908), e( 892,  918), e( 888,  943)],
                [e( 901,  902), e( 895,  912), e( 894,  916), e( 893,  929)],
                [e( 910,  890), e( 913,  900), e( 905,  910), e( 901,  904)],
                [e( 905,  876), e( 905,  889), e( 912,  883), e( 912,  884)],
                [e( 894,  887), e( 894,  886), e( 903,  874), e( 912,  888)],
            ],
            [
                [e( 898,  893), e( 937,  885), e( 913,  933), e( 898,  938)],
                [e( 924,  912), e( 884,  935), e( 896,  951), e( 864,  979)],
                [e( 890,  918), e( 920,  903), e( 897,  941), e( 902,  948)],
                [e( 896,  932), e( 894,  941), e( 891,  951), e( 886,  951)],
                [e( 913,  922), e( 912,  934), e( 901,  931), e( 898,  934)],
                [e( 921,  904), e( 924,  922), e( 922,  915), e( 900,  913)],
                [e( 899,  867), e( 917,  850), e( 917,  868), e( 912,  890)],
                [e( 889,  864), e( 883,  868), e( 885,  885), e( 906,  871)],
            ],
        ]),
        king: Pst([
            [e(  -9,  -81), e(   8,  -47), e(  14,  -30), e(  -8,   -6), e(  -3,  -23), e(   6,  -16), e(  10,  -16), e(  -1,  -82)],
            [e( -20,  -24), e(  13,   10), e(   0,   16), e(  39,   19), e(  14,   30), e(  18,   36), e(  28,   21), e(   9,  -12)],
            [e( -28,  -11), e(  47,   12), e(  14,   39), e(  -2,   52), e(  10,   57), e(  57,   44), e(  41,   26), e(  10,   -8)],
            [e( -25,  -16), e( -12,   16), e( -18,   43), e( -63,   65), e( -74,   67), e( -31,   46), e( -27,   22), e( -69,    1)],
            [e( -33,  -24), e( -10,    5), e( -42,   35), e( -90,   58), e( -82,   58), e( -35,   32), e( -32,   11), e( -88,    0)],
            [e(   5,  -31), e(  45,   -7), e( -10,   18), e( -28,   32), e( -18,   28), e(  -4,   19), e(  31,   -5), e( -12,  -12)],
            [e(  55,  -48), e(  49,  -25), e(  27,   -7), e(   0,    3), e(   0,    6), e(  13,   -3), e(  51,  -25), e(  42,  -37)],
            [e(  45,  -70), e(  59,  -53), e(  37,  -29), e( -32,  -15), e(  20,  -33), e(  -5,  -19), e(  50,  -46), e(  47,  -61)],
        ]),
    },
    mobility: Mobility {
        pawn: [e(-16, 8), e(-8, 30), e(1, 35), e(27, 16), e(-1, 50)],
        knight: [e(-63, 33), e(-50, 26), e(-44, 32), e(-40, 28), e(-39, 33), e(-39, 39), e(-38, 39), e(-40, 39), e(-38, 34)],
        bishop: [e(-68, -18), e(-60, -4), e(-55, 4), e(-49, 16), e(-42, 29), e(-35, 42), e(-30, 46), e(-28, 54), e(-28, 61), e(-28, 61), e(-27, 59), e(-24, 59), e(-18, 64), e(0, 51)],
        rook: [e(-133, 94), e(-122, 114), e(-120, 117), e(-116, 121), e(-117, 126), e(-114, 129), e(-111, 131), e(-108, 135), e(-109, 140), e(-109, 147), e(-108, 148), e(-107, 152), e(-109, 155), e(-104, 157), e(-113, 157)],
        queen: [e(-36, 7), e(-41, 47), e(-51, 155), e(-53, 190), e(-51, 196), e(-48, 201), e(-45, 211), e(-47, 230), e(-46, 239), e(-45, 241), e(-45, 251), e(-44, 258), e(-46, 267), e(-44, 274), e(-44, 279), e(-45, 285), e(-41, 288), e(-45, 299), e(-44, 307), e(-41, 307), e(-31, 304), e(-29, 308), e(-22, 305), e(-20, 310), e(8, 292), e(29, 296), e(4, 302), e(10, 304)],
        king: [e(8, -2), e(-28, -38), e(-28, -20), e(-33, -1), e(-33, 1), e(-36, 6), e(-24, 3), e(-22, 6), e(-21, 7)],
    },
    passed_pawns: KingRelativePst([
        [
            [e(   0,    0), e(   0,    0), e(   0,    0), e(   0,    0)],
            [e(  10,   96), e(  27,   91), e(  27,   73), e(  37,   63)],
            [e(  21,  132), e(  41,  117), e(  31,  100), e(  17,   69)],
            [e(  28,   62), e(  20,   52), e(  27,   37), e(  15,   41)],
            [e(  16,   31), e(   1,   19), e( -11,   17), e(  -7,   14)],
            [e(   2,    4), e( -16,   11), e( -16,    7), e( -14,   -7)],
            [e(  -2,    4), e(  -5,    6), e(  -7,    1), e( -12,  -15)],
            [e(   0,    0), e(   0,    0), e(   0,    0), e(   0,    0)],
        ],
        [
            [e(   0,    0), e(   0,    0), e(   0,    0), e(   0,    0)],
            [e( -34,  102), e( -13,   96), e(  17,   89), e(  34,   67)],
            [e( -63,  168), e( -43,  149), e(   2,  133), e(  13,   97)],
            [e( -35,   92), e( -20,   84), e(   9,   69), e(  -1,   49)],
            [e( -33,   53), e( -11,   49), e( -18,   36), e( -15,   25)],
            [e( -13,   13), e(  -2,   13), e( -28,   11), e( -15,    4)],
            [e( -22,   11), e(   9,    1), e(  -7,   -1), e(  -2,   -8)],
            [e(   0,    0), e(   0,    0), e(   0,    0), e(   0,    0)],
        ],
    ]),
    bishop_pair: e(22, 64),
    rook_on_open_file: e(36, 6),
    rook_on_semiopen_file: e(13, 4),
    virtual_queen_mobility: [e(6, -1), e(51, 30), e(43, 23), e(41, 7), e(37, 8), e(38, 4), e(34, 3), e(32, 5), e(27, 6), e(20, 7), e(14, 10), e(5, 12), e(-2, 14), e(-14, 16), e(-25, 15), e(-33, 16), e(-44, 12), e(-47, 12), e(-46, 6), e(-34, -1), e(-27, -7), e(-27, -12), e(-23, -17), e(-9, -25), e(-9, -33), e(-3, -39), e(-5, -40), e(-3, -33)],
    king_ring_attacks: [e(73, -27), e(67, -14), e(55, -7), e(22, 0), e(-24, 18), e(-70, 30), e(-74, 26), e(-43, -18), e(-8, -8)],
};

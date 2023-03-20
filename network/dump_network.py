import sys, json, numpy as np

ACTIVATION_RANGE = 127
WEIGHT_SCALE = 64

STATE = { k: np.array(v) for k, v in json.load(sys.stdin).items() }

def dump_tensor(tensor: np.ndarray, scale: float) -> str:
    if len(tensor.shape) == 0:
        return str(round(tensor.item() * scale))
    return f"[{','.join(dump_tensor(t, scale) for t in tensor)}]"

def dump_struct(type: str, fields: dict[str, str]) -> str:
    return f"{type}{{{','.join(f'{k}:{v}' for k, v in fields.items())}}}"

def dump_ft():
    weights = STATE["ft.weight"].transpose()
    biases = STATE["ft.bias"]
    return dump_struct("BitLinear", {
        "weights": dump_tensor(weights, ACTIVATION_RANGE),
        "biases": dump_tensor(biases, ACTIVATION_RANGE)
    })

def dump_linear(field: str):
    weights = STATE[f"{field}.weight"]
    biases = STATE[f"{field}.bias"]
    return dump_struct("Linear", {
        "weights": dump_tensor(weights, WEIGHT_SCALE),
        "biases": dump_tensor(biases, WEIGHT_SCALE * ACTIVATION_RANGE)
    })

print(dump_struct("Nnue", {
    "ft": dump_ft(),
    "l1": dump_linear("out")
}))

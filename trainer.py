from typing import List, OrderedDict, Tuple, Dict
import os, torch, time, numpy as np
from torch import Tensor, nn, optim
from torch.utils.data import Dataset, DataLoader
from dataclasses import dataclass
from argparse import ArgumentParser
from struct import Struct

FEATURES = 768
FT_OUT   = 32
L1_OUT   = 1

ACTIVATION_RANGE = 127
WEIGHT_SCALE = 64
LINEAR_MIN_WEIGHT = -128 / WEIGHT_SCALE
LINEAR_MAX_WEIGHT = 127 / WEIGHT_SCALE

class Nnue(nn.Module):
    ft: nn.Linear
    l1: nn.Linear

    def __init__(self):
        super(Nnue, self).__init__()

        self.ft = nn.Linear(FEATURES, FT_OUT)
        self.l1 = nn.Linear(FT_OUT * 2, L1_OUT)

    def forward(self, features: List[Tensor]) -> Tensor:
        stm_features, sntm_features = features
        stm         = self.ft(stm_features)
        sntm        = self.ft(sntm_features)
        accumulator = torch.cat([stm, sntm], dim=1)
        
        l1_in = torch.clamp(accumulator, 0.0, 1.0)
        l1    = self.l1(l1_in)
        return l1

MAX_FEATURES = 32
FEATURES_STRUCT = Struct("<" + "H" * MAX_FEATURES)
DATASET_ENTRY_SIZE = FEATURES_STRUCT.size * 2 + 1

class PositionSet(Dataset):
    data: bytes

    def __init__(self, path: str):
        with open(path, "rb") as f:
            self.data = f.read()

    def __getitem__(self, index: int) -> Tuple[List[Tensor], Tensor]:
        def decode_features(data: bytes, index: int) -> Tensor:
            raw = FEATURES_STRUCT.unpack_from(data, index)
            features = np.zeros(FEATURES, dtype=np.float32)
            for feature in raw:
                if feature == 65535:
                    break
                features[feature] = 1
            return torch.as_tensor(features)
        field_index = index * DATASET_ENTRY_SIZE
        stm_features = decode_features(self.data, field_index)
        field_index += FEATURES_STRUCT.size
        sntm_features = decode_features(self.data, field_index)
        field_index += FEATURES_STRUCT.size
        win_rate = torch.tensor([self.data[field_index] / 255])
        field_index += 1
        return [stm_features, sntm_features], win_rate

    def __len__(self) -> int:
        return len(self.data) // DATASET_ENTRY_SIZE

@dataclass
class Checkpoint:
    model_state_dict: OrderedDict[str, Tensor]
    optimizer_state_dict: dict
    epoch: int
    loss: float

class TrainerState:
    model: Nnue
    optimizer: optim.Optimizer
    epoch: int
    loss: float

    def __init__(self):
        self.model = Nnue()
        self.optimizer = optim.Adam(self.model.parameters())
        self.epoch = 0
        self.loss = 0
    
    def load(self, path: str):
        checkpoint: Checkpoint = torch.load(path)
        self.model.load_state_dict(checkpoint.model_state_dict)
        self.optimizer.load_state_dict(checkpoint.optimizer_state_dict)
        self.epoch = checkpoint.epoch
        self.loss = checkpoint.loss

    def save(self, path: str):
        checkpoint = Checkpoint(
            model_state_dict=self.model.state_dict(),
            optimizer_state_dict=self.optimizer.state_dict(),
            epoch=self.epoch,
            loss=self.loss
        )
        torch.save(checkpoint, path)

    def dump(self) -> str:
        state = self.model.state_dict()
        def dump_tensor(tensor: Tensor, scale: float) -> str:
            if len(tensor.shape) == 0:
                return str(round(tensor.item() * scale))
            return f"[{','.join(dump_tensor(t, scale) for t in tensor)}]"
        def dump_struct(type: str, fields: Dict[str, str]) -> str:
            return f"{type}{{{','.join(f'{k}:{v}' for k, v in fields.items())}}}"
        def dump_ft():
            weights = state["ft.weight"].transpose(0, 1)
            biases = state["ft.bias"]
            return dump_struct("BitLinear", {
                "weights": dump_tensor(weights, ACTIVATION_RANGE),
                "biases": dump_tensor(biases, ACTIVATION_RANGE)
            })
        def dump_linear(field: str):
            weights = state[f"{field}.weight"]
            biases = state[f"{field}.bias"]
            return dump_struct("Linear", {
                "weights": dump_tensor(weights, WEIGHT_SCALE),
                "biases": dump_tensor(biases, WEIGHT_SCALE * ACTIVATION_RANGE)
            })
        return dump_struct("Nnue", {
            "ft": dump_ft(),
            "l1": dump_linear("l1")
        })

BATCHES_PER_PRINT = 500

class Trainer:
    checkpoint_dir: str
    state: TrainerState
    dataset: PositionSet
    dataloader: DataLoader[Tuple[List[Tensor], Tensor]]
    criterion: nn.MSELoss

    def __init__(self, checkpoint_dir: str, state: TrainerState, dataset: PositionSet):
        self.checkpoint_dir = checkpoint_dir
        self.state = state
        self.dataset = dataset
        self.dataloader = DataLoader(dataset, batch_size=32, shuffle=True)
        self.criterion = nn.MSELoss()

    def train(self):
        self.state.model.train()
        while True:
            self.state.epoch += 1
            self.loss = self.do_epoch()
            checkpoint_name = f"epoch-{self.state.epoch}.tar"
            self.state.save(os.path.join(self.checkpoint_dir, checkpoint_name))
            print(f"\rEpoch {self.state.epoch} - Loss: {self.loss}")

    def do_epoch(self) -> float:
        epoch_loss = 0
        positions = 0
        running_loss = 0
        running_loss_positions = 0
        running_loss_start = time.time()
        for batch, (inputs, labels) in enumerate(self.dataloader):
            loss = self.train_step(inputs, labels)
            batch_size = inputs[0].shape[0]
            epoch_loss += loss * batch_size
            running_loss += loss * batch_size
            positions += batch_size
            running_loss_positions += batch_size

            if batch % BATCHES_PER_PRINT == 0:
                now = time.time()
                speed = running_loss_positions / (now - running_loss_start)
                running_loss /= running_loss_positions
                print(f"Epoch {self.state.epoch} - Loss: {running_loss:>7f} [{positions:>5d}/{len(dataset):>5d}] [{speed:>2f} pos/s]")
                running_loss = 0
                running_loss_positions = 0
                running_loss_start = now
        return epoch_loss / len(self.dataset)

    def train_step(self, inputs: List[Tensor], labels: Tensor) -> float:
        outputs = torch.sigmoid(self.state.model(inputs))
        self.state.optimizer.zero_grad()
        loss = self.criterion(outputs, labels)
        loss.backward()
        self.state.optimizer.step()
        self.clamp_model()
        return loss.item()

    def clamp_model(self):
        def clamp(tensor: Tensor, min: float, max: float):
            tensor.data = tensor.data.clamp(min, max)
        clamp(self.state.model.l1.weight, LINEAR_MIN_WEIGHT, LINEAR_MAX_WEIGHT)

if __name__ == "__main__":
    parser = ArgumentParser(description="Trainer for Tantabus")
    subparsers = parser.add_subparsers(dest="command", required=True, help="A subcommand")
    train_subparser = subparsers.add_parser("train", help="Train a network")
    train_subparser.add_argument("cp_dir", help="Path to the checkpoint directory")
    train_subparser.add_argument("dataset", help="Path to a dataset")
    train_subparser.add_argument("--checkpoint", help="Checkpoint in the checkpoint directory")
    dump_subparser = subparsers.add_parser("dump", help="Dump a network")
    dump_subparser.add_argument("checkpoint", help="Path to a checkpoint")

    args = parser.parse_args()
    if args.command == "train":
        state = TrainerState()
        if args.checkpoint != None:
            checkpoint_path = os.path.join(args.cp_dir, args.checkpoint)
            print(f"Loading checkpoint {checkpoint_path}")
            state.load(checkpoint_path)
        dataset = PositionSet(args.dataset)
        trainer = Trainer(args.cp_dir, state, dataset)
        trainer.train()
    if args.command == "dump":
        state = TrainerState()
        state.load(args.checkpoint)
        print(state.dump())

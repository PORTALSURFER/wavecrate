$ErrorActionPreference = "Stop"

$dataset = "./tmp/training_dataset/scripts_dataset"
$out = "./tmp/training_dataset/model.json"

cargo run --bin sempal-train-mlp -- --hybrid --dataset $dataset --out $out

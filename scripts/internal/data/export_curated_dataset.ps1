$ErrorActionPreference = "Stop"

$dataset = "D:/music-production/samples/trainingset"
$outDir = "./tmp/training_dataset/scripts_dataset"

cargo run --bin wavecrate-dataset-export-curated -- --dataset $dataset --out $outDir --hybrid

$ErrorActionPreference = "Stop"

$dataset = "D:/music-production/samples/trainingset"
$outDir = "./tmp/training_dataset/scripts_dataset"

cargo run --bin sempal-dataset-export-curated -- --dataset $dataset --out $outDir --hybrid

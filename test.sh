# `formatter` is not included
features=("completion" "folding" "highlight" "hover" "linked_editing" "links" "matching_tag_position" "rename" "selection_range" "symbols")

for feature in "${features[@]}"
do
    echo "cargo test --features $feature"
    cargo test --features $feature || exit 1
done

How to Compile and Deploy:

Make sure you have wasm-bindgen installed

`cargo install wasm-bindgen-cli`

Enter the project root

This project includes a src/ and cargo.toml , compile them with

`cargo build --release --target wasm32-unknown-unknown`

Doing this will generate a target folder, now run 

`wasm-bindgen --no-typescript --target web \
    --out-dir ./out/ \
    --out-name "webrat" \
    ./target/wasm32-unknown-unknown/release/webrat.wasm`

This will create an out/ folder in which will be  .js and .wasm files

Put these js and wasm files in the same folder as the provided index.html , now you can deploy this website , don't forget the assets/ folder with fonts if you need them (the default bevy font is not very good)

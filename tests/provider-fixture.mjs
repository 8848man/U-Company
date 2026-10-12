const prompt = process.argv.at(-1);
let stdinClosed = false;
process.stdin.on('end', () => {
  stdinClosed = true;
  console.log(JSON.stringify({ type: 'item.completed', item: { type: 'agent_message', text: `PASS:${prompt}:${process.cwd()}:${stdinClosed}:${Boolean(process.env.PATH)}` } }));
});
process.stdin.resume();
setTimeout(() => {
  if (!stdinClosed) {
    console.error('fixture stdin did not close');
    process.exit(4);
  }
}, 1500).unref();

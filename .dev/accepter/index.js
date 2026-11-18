const express = require("express");

const app = express();
const port = 3013;

// In-memory state to track request counts
let state = {
    perSecond: 0,
    perMinute: 0,
    total: 0
};

// Log and reset per-second counter every 1,000 ms (1 second)
setInterval(() => {
    console.log(`[3s] POST requests: ${state.perSecond} | Total: ${state.total}`);
    state.perSecond = 0;
}, 3000);

// Log and reset per-minute counter every 60,000 ms (1 minute)
setInterval(() => {
    console.log(`[1m] POST requests: ${state.perMinute} | Total: ${state.total}`);
    state.perMinute = 0;
}, 60000);

app.get('/', (req, res) => {
    res.send('Hello World!');
});

app.post('/', (req, res) => {
    // Increment counters on every POST request
    state.perSecond++;
    state.perMinute++;
    state.total++;

    res.send('working');
});

app.listen(port, () => {
    console.log(`Server running at http://localhost:${port}/`);
});

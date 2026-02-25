// AI Worker for Prime-Time Node
// Manages thermodynamic account decisions

self.onmessage = function(e) {
    const { type, payload } = e.data;

    if (type === 'INIT') {
        self.nodeId = payload.id;
        self.config = payload.config;
    } else if (type === 'TICK') {
        decideAction(payload);
    }
};

function decideAction(state) {
    const me = state.participants.find(p => p.id === self.nodeId);
    if (!me || !me.isOnline) return;

    // Thermodynamic Logic:
    // If we have excess Prime Value (High Entropy Potential), distribute to lower value nodes to generate Positive Entropy.
    // Avoid hoarding if it doesn't lead to a Mint.

    // 1. Check if we can Mint (Priority #1)
    // The main thread handles minting automatically if we have enough value,
    // but maybe we want to hold onto value for a specific Heuristic Mint?
    // For now, let's assume standard minting is automatic.

    // 2. Entropy Generation (Priority #2)
    // If we have significantly more value than the average, or a specific threshold, consider transferring.

    const others = state.participants.filter(p => p.id !== self.nodeId);
    if (others.length === 0) return;

    // Calculate average Prime Value
    const totalValue = state.participants.reduce((acc, p) => acc + p.primeValue, 0);
    const avgValue = totalValue / state.participants.length;

    // Decision Thresholds
    const WEALTHY_THRESHOLD = avgValue * 1.5;
    const POOR_THRESHOLD = avgValue * 0.5;

    if (me.primeValue > WEALTHY_THRESHOLD) {
        // We are wealthy. Look for a poor node to transfer to (Generates Positive Entropy).
        // Target the poorest node for maximum entropy generation?
        // Or a random poor node to spread liquidity?

        const poorNodes = others.filter(p => p.primeValue < POOR_THRESHOLD);

        if (poorNodes.length > 0) {
            // Sort by lowest value to maximize entropy gradient
            poorNodes.sort((a, b) => a.primeValue - b.primeValue);
            const target = poorNodes[0];

            // Decide amount and type
            // For now, let's transfer a 'DEGREE' if we have enough, or 'MINUTE'
            // We need to know the cost of units.
            // Passed in config? Or estimated?
            // Let's assume we can ask the main thread or know the standard.
            // For simplicity, let's try to transfer a standard 'DEGREE' (1800 counts approx, depends on prime).

            // Send action request
            // We don't have the exact unit costs here unless passed in state.
            // Let's assume the main thread handles validity checks.

            // Probability check to avoid spamming transfers every tick
            if (Math.random() < 0.05) {
                self.postMessage({
                    type: 'ACTION_TRANSFER',
                    payload: {
                        targetId: target.id,
                        unitType: 'DEGREE' // Good balance of value
                    }
                });
            }
        }
    }
}

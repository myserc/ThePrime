const fs = require('fs');
const test = require('node:test');
const assert = require('node:assert');

// 1. Read the production code
const htmlContent = fs.readFileSync('casa.html', 'utf-8');

// 2. Extract the load() function body
const loadMatch = htmlContent.match(/load\(\)\s*\{([\s\S]*?)\},\s*clear\(\)/);
if (!loadMatch) {
    throw new Error("Could not extract Persistence.load() from casa.html");
}
const loadBody = loadMatch[1];

// 3. Create a wrapper function that receives the necessary dependencies
const loadFunction = new Function('localStorage', 'state', 'console', `
    // Define the context inside the function, including 'this.KEY'
    const context = {
        KEY: 'prime_time_v4_state'
    };

    // Bind the function body to run with our context
    const fn = function() {
        ${loadBody}
    };
    return fn.call(context);
`);

test('Persistence.load() tests', async (t) => {

    await t.test('returns false if localStorage returns null (no save)', () => {
        const mockLocalStorage = {
            getItem: (key) => null
        };
        const mockState = {};
        const mockConsole = { error: () => {} };

        const result = loadFunction(mockLocalStorage, mockState, mockConsole);
        assert.strictEqual(result, false);
    });

    await t.test('successfully loads state from valid JSON', () => {
        const savedData = {
            participants: [{ id: 1 }],
            books: [{ id: 100 }],
            transfers: [{ id: 500 }],
            totalScarcity: 1000,
            netEntropy: 50,
            simTime: 12345,
            idCounter: 1001,
            isAuto: true,
            systemSurplus: 10
        };

        const mockLocalStorage = {
            getItem: (key) => {
                assert.strictEqual(key, 'prime_time_v4_state');
                return JSON.stringify(savedData);
            }
        };

        const mockState = {}; // This will be mutated by the function
        const mockConsole = { error: () => {} };

        const result = loadFunction(mockLocalStorage, mockState, mockConsole);

        assert.strictEqual(result, true);
        assert.deepStrictEqual(mockState.participants, savedData.participants);
        assert.deepStrictEqual(mockState.books, savedData.books);
        assert.deepStrictEqual(mockState.transfers, savedData.transfers);
        assert.strictEqual(mockState.totalScarcity, savedData.totalScarcity);
        assert.strictEqual(mockState.netEntropy, savedData.netEntropy);
        assert.strictEqual(mockState.simTime, savedData.simTime);
        assert.strictEqual(mockState.idCounter, savedData.idCounter);
        assert.strictEqual(mockState.isAuto, savedData.isAuto);
        assert.strictEqual(mockState.systemSurplus, savedData.systemSurplus);
    });

    await t.test('falls back to default values for missing fields in JSON', () => {
        const savedData = {
            // Missing all optional fields
        };

        const mockLocalStorage = {
            getItem: (key) => JSON.stringify(savedData)
        };

        const mockState = {};
        const mockConsole = { error: () => {} };

        const result = loadFunction(mockLocalStorage, mockState, mockConsole);

        assert.strictEqual(result, true);
        assert.deepStrictEqual(mockState.participants, []);
        assert.deepStrictEqual(mockState.books, []);
        assert.deepStrictEqual(mockState.transfers, []);
        assert.strictEqual(mockState.totalScarcity, 0);
        assert.strictEqual(mockState.netEntropy, 0);
        assert.strictEqual(mockState.simTime, 0);
        assert.strictEqual(mockState.idCounter, 1000);
        assert.strictEqual(mockState.isAuto, false);
        assert.strictEqual(mockState.systemSurplus, 0);
    });

    await t.test('returns false and logs error on corrupted JSON', () => {
        const mockLocalStorage = {
            getItem: (key) => "{ corrupted: json"
        };
        const mockState = {};

        let errorLogged = false;
        let loggedMessage = "";
        const mockConsole = {
            error: (msg, e) => {
                errorLogged = true;
                loggedMessage = msg;
            }
        };

        const result = loadFunction(mockLocalStorage, mockState, mockConsole);

        assert.strictEqual(result, false);
        assert.strictEqual(errorLogged, true);
        assert.strictEqual(loggedMessage, "Save file corrupted");
    });
});

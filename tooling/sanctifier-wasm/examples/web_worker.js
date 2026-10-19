/**
 * Web Worker Example for @sanctifier/wasm
 * 
 * This demonstrates how to offload WASM analysis to a background worker
 * to avoid blocking the main UI thread during intensive static analysis.
 */

import { analyze, version } from '@sanctifier/wasm';

// Handle incoming messages from the main thread
self.onmessage = async (event) => {
    const { id, type, source, config } = event.data;

    try {
        let result;
        
        switch (type) {
            case 'ANALYZE':
                // Run standard analysis
                result = analyze(source);
                break;
                
            case 'GET_VERSION':
                result = version();
                break;
                
            default:
                throw new Error(`Unknown analysis task type: ${type}`);
        }

        // Send results back to the main thread
        self.postMessage({ id, status: 'success', data: result });
        
    } catch (error) {
        // Send errors back
        self.postMessage({ 
            id, 
            status: 'error', 
            error: error.message || 'Analysis failed'
        });
    }
};

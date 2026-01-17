import React, { useState } from 'react';

export default function App() {
    const [count, setCount] = useState(0);

    return (
        <div style={{ fontFamily: 'sans-serif', textAlign: 'center', padding: '50px' }}>
            <h1>⚡ Nexus React Basic</h1>
            <p>This is a basic React app running on Nexus.</p>
            <div style={{ marginTop: '20px' }}>
                <button
                    onClick={() => setCount(c => c + 1)}
                    style={{ padding: '10px 20px', fontSize: '16px', cursor: 'pointer' }}
                >
                    Count is {count}
                </button>
            </div>
        </div>
    );
}

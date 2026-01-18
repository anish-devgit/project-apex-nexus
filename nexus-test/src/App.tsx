import React, { useState } from 'react';

export default function App() {
    const [count, setCount] = useState(0);

    return (
        <div>
            <h1>Nexus Dev Server Test</h1>
            <button onClick={() => setCount(c => c + 1)}>
                Count: {count}
            </button>
        </div>
    );
}

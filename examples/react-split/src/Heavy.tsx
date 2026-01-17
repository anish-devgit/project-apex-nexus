import React from 'react';

export default function HeavyComponent() {
    return (
        <div style={{
            padding: '40px',
            background: '#f0f0f0',
            border: '2px dashed #333',
            borderRadius: '8px'
        }}>
            <h2>📦 Heavy Component Loaded!</h2>
            <p>This component was loaded from a separate <code>chunk-src-heavy.js</code> file.</p>
        </div>
    );
}

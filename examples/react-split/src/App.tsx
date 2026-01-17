import React, { useState, Suspense } from 'react';

// Dynamic Import
const HeavyComponent = React.lazy(() => import('./Heavy'));

export default function App() {
    const [show, setShow] = useState(false);

    return (
        <div style={{ fontFamily: 'sans-serif', textAlign: 'center', padding: '50px' }}>
            <h1>⚡ Nexus Code Splitting</h1>
            <p>Click the button below to load a chunk dynamically.</p>

            <button
                onClick={() => setShow(true)}
                style={{ padding: '10px 20px', fontSize: '16px', cursor: 'pointer', marginBottom: '20px' }}
            >
                Load Heavy Component
            </button>

            {show && (
                <Suspense fallback={<div>Loading chunk...</div>}>
                    <HeavyComponent />
                </Suspense>
            )}
        </div>
    );
}

// Watermark Heatmap Chart
let watermarkHeatmapChart = null;

// Function to fetch and update watermark heatmap
window.updateWatermarkHeatmap = async function(fromDate, toDate) {
    try {
        const response = await fetch(`/watermarks?from=${fromDate}&to=${toDate}`);
        const data = await response.json();
        
        console.log('Watermark data received:', data);
        
        if (data.watermarks && data.watermarks.length > 0) {
            const minWatermark = data.min_watermark || 0;
            const maxWatermark = data.max_watermark || 40;
            renderWatermarkHeatmap(data.watermarks, minWatermark, maxWatermark);
        } else {
            // Show empty state
            renderEmptyHeatmap();
        }
    } catch (error) {
        console.error('Error fetching watermark data:', error);
        renderEmptyHeatmap();
    }
};

function renderWatermarkHeatmap(watermarkData, minWatermark = 20, maxWatermark = 40) {
    const ctx = document.getElementById('watermarkHeatmapChart');
    if (!ctx) {
        console.error('Canvas element not found');
        return;
    }

    // Generate all 52 weeks for the full year
    const xLabels = [];
    const today = new Date();
    const yearAgo = new Date(today);
    yearAgo.setDate(today.getDate() - 365);
    
    for (let i = 0; i < 52; i++) {
        const weekStart = new Date(yearAgo);
        weekStart.setDate(yearAgo.getDate() + (i * 7));
        const weekLabel = `W${String(i + 1).padStart(2, '0')}-${String(weekStart.getMonth() + 1).padStart(2, '0')}/${String(weekStart.getDate()).padStart(2, '0')}`;
        xLabels.push(weekLabel);
    }
    
    // Fixed Y-axis: 20-40 with scale of 1
    const yLabels = [];
    for (let i = 20; i < 40; i++) {
        yLabels.push(`${i}`);
    }
    
    // Create a complete matrix with all combinations, filling missing values with 0
    const dataMap = new Map();
    watermarkData.forEach(point => {
        dataMap.set(`${point.x}|${point.y}`, point.value);
    });

    const matrixData = [];
    xLabels.forEach((xLabel, xIndex) => {
        yLabels.forEach((yLabel, yIndex) => {
            const key = `${xLabel}|${yLabel}`;
            const value = dataMap.get(key) || 0;
            matrixData.push({
                x: xLabel,
                y: yLabel,
                v: value
            });
        });
    });

    // Find max value for color scaling
    const maxValue = Math.max(...watermarkData.map(d => d.value), 1);

    // Destroy existing chart if it exists
    if (watermarkHeatmapChart) {
        watermarkHeatmapChart.destroy();
    }

    // Create new heatmap chart
    watermarkHeatmapChart = new Chart(ctx, {
        type: 'matrix',
        data: {
            datasets: [{
                label: 'Watermark Hits',
                data: matrixData,
                backgroundColor(context) {
                    const value = context.dataset.data[context.dataIndex]?.v || 0;
                    if (value === 0) {
                        return 'rgba(229, 231, 235, 0.3)'; // Light gray for empty cells
                    }
                    
                    // More gradual heat gradient: light yellow → yellow → orange → red
                    // At value 1, start with very light yellow
                    const intensity = Math.min(value / maxValue, 1.0);
                    
                    // Use a smoother curve for better gradient
                    const adjustedIntensity = Math.pow(intensity, 0.7); // Soften the gradient
                    
                    if (adjustedIntensity < 0.25) {
                        // Very light yellow to yellow (value 1 starts here)
                        const t = adjustedIntensity / 0.25;
                        const r = 255;
                        const g = Math.round(255 - (t * 30)); // 255 → 225 (very subtle)
                        const b = Math.round(200 - (t * 200)); // 200 → 0 (remove blue tint)
                        return `rgb(${r}, ${g}, ${b})`;
                    } else if (adjustedIntensity < 0.5) {
                        // Yellow to orange
                        const t = (adjustedIntensity - 0.25) / 0.25;
                        const r = 255;
                        const g = Math.round(225 - (t * 60)); // 225 → 165
                        const b = 0;
                        return `rgb(${r}, ${g}, ${b})`;
                    } else if (adjustedIntensity < 0.75) {
                        // Orange to red-orange
                        const t = (adjustedIntensity - 0.5) / 0.25;
                        const r = 255;
                        const g = Math.round(165 - (t * 65)); // 165 → 100
                        const b = 0;
                        return `rgb(${r}, ${g}, ${b})`;
                    } else {
                        // Red-orange to deep red
                        const t = (adjustedIntensity - 0.75) / 0.25;
                        const r = 255;
                        const g = Math.round(100 - (t * 100)); // 100 → 0
                        const b = 0;
                        return `rgb(${r}, ${g}, ${b})`;
                    }
                },
                borderColor: 'rgba(255, 255, 255, 0.8)',
                borderWidth: 1,
                width: ({ chart }) => {
                    const chartArea = chart.chartArea || {};
                    const height = chartArea.bottom - chartArea.top;
                    const cellSize = (height / yLabels.length) - 2;
                    return cellSize;
                },
                height: ({ chart }) => {
                    const chartArea = chart.chartArea || {};
                    const height = chartArea.bottom - chartArea.top;
                    const cellSize = (height / yLabels.length) - 2;
                    return cellSize;
                }
            }]
        },
        options: {
            responsive: true,
            maintainAspectRatio: false,
            plugins: {
                legend: {
                    display: false
                },
                tooltip: {
                    callbacks: {
                        title() {
                            return '';
                        },
                        label(context) {
                            const v = context.dataset.data[context.dataIndex];
                            
                            if (v.v === 0) {
                                return [
                                    `Watermark: ${v.y}`,
                                    `Count: 0`
                                ];
                            }
                            return [
                                `Watermark: ${v.y}`,
                                `Count: ${v.v}`
                            ];
                        }
                    }
                }
            },
            scales: {
                x: {
                    type: 'category',
                    labels: xLabels,
                    offset: true,
                    ticks: {
                        display: false  // Hide X-axis labels
                    },
                    grid: {
                        display: false
                    }
                },
                y: {
                    type: 'category',
                    labels: yLabels,
                    offset: true,
                    ticks: {
                        font: {
                            size: 11
                        }
                    },
                    grid: {
                        display: false
                    }
                }
            }
        }
    });
}

function renderEmptyHeatmap() {
    const container = document.getElementById('watermark-heatmap');
    if (container) {
        const canvas = container.querySelector('canvas');
        if (canvas) {
            canvas.style.display = 'none';
        }
        
        // Show placeholder message
        let placeholder = container.querySelector('.placeholder-text');
        if (!placeholder) {
            placeholder = document.createElement('div');
            placeholder.className = 'placeholder-text';
            container.appendChild(placeholder);
        }
        placeholder.textContent = 'No watermark data available for this period';
        placeholder.style.display = 'block';
    }
    
    // Destroy existing chart
    if (watermarkHeatmapChart) {
        watermarkHeatmapChart.destroy();
        watermarkHeatmapChart = null;
    }
}

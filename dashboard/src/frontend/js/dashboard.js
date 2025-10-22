let myLineChart = null;

document.addEventListener('DOMContentLoaded', function () {
  function initialisePnlChart() {
    // Initialize the chart
    const canvas = document.getElementById("pnlChart");
    if (!canvas) {
      console.error("Canvas element 'pnlChart' not found");
      return;
    }

    const ctx = canvas.getContext('2d');
    console.log("Initializing chart...");
    myLineChart = new Chart(ctx, {
      type: 'line',
      data: {
        datasets: [],
      },
      options: {
        scales: {
          x: {
            type: 'time',
            time: {
              unit: 'day',
              tooltipFormat: 'MMM d, yyyy',
              displayFormats: {
                day: 'MMM d'
              }
            },
            title: {
              display: true,
              text: 'Date',
            },
          },
          y: {
            beginAtZero: false, // Allow negative values
            title: {
              display: true,
              text: 'PnL',
            },
          },
        },
        plugins: {
          tooltip: {
            callbacks: {
              label: function (context) {
                const label = context.dataset.label || '';
                const value = context.parsed.y;

                // Get the original data point from the dataset's metadata
                const dataPoint = context.dataset.originalData?.[context.dataIndex];

                // Start with the cumulative PnL info
                let tooltipText = [`${label} (Cumulative): $${value.toFixed(2)}`];

                // Add additional information if available
                if (dataPoint) {
                  if (dataPoint.pnl) tooltipText.push(`Trade PnL: $${parseFloat(dataPoint.pnl).toFixed(2)}`);
                  if (dataPoint.exit_date) tooltipText.push(`Exit: ${dataPoint.exit_date}`);
                  if (dataPoint.roi) tooltipText.push(`ROI: ${parseFloat(dataPoint.roi).toFixed(2)}%`);
                }

                return tooltipText;
              }
            }
          }
        }
      },
    });
    console.log("Chart initialized");
  }

  initialisePnlChart();

});

window.updatePnlChart = async function (start_date, end_date) {
  try {
    const params = new URLSearchParams();
    // Use 'from' and 'to' instead of 'start_date' and 'end_date'
    params.append('from', start_date);
    params.append('to', end_date);
    // Add is_active parameter if needed (assuming you want closed positions)
    params.append('is_active', 'false');

    const url = `/performance?${params.toString()}`;
    const response = await fetch(url, { method: 'GET' });

    if (!response.ok) {
      // Try to get error details from response
      let errorDetails = '';
      try {
        const errorText = await response.text();
        console.log("Error response body:", errorText);
        errorDetails = errorText;
      } catch (e) {
        console.log("Could not extract error details", e);
        errorDetails = 'Could not extract error details';
      }

      throw new Error(`HTTP error! Status: ${response.status}, Details: ${errorDetails}`);
    }


    const data = await response.json();
    console.log("Received data:", data);

    // Check if we have data before updating the chart
    if (data && data.performance && data.performance.response) {
      // Calculate total PnL across all strategies
      const totalPnl = data.performance.response.reduce((sum, item) => {
        return sum + parseFloat(item.pnl || 0);
      }, 0);

      // Count winning trades
      const winningTrades = data.performance.response.filter(item =>
        parseFloat(item.pnl || 0) > 0
      ).length;

      // Calculate win rate
      const totalTrades = data.performance.response.length;
      const winRate = totalTrades > 0 ? (winningTrades / totalTrades * 100) : 0;

      // Update dashboard elements
      const totalPnlElement = document.getElementById('total-pnl');
      if (totalPnlElement) {
        totalPnlElement.textContent = `$${totalPnl.toFixed(2)}`;
      }

      // Update PnL trend percentage
      // This would require comparing with previous period data
      // For now, let's calculate a simple percentage of winning vs total trades
      const pnlTrendElement = totalPnlElement?.parentElement?.querySelector('.trend');
      if (pnlTrendElement) {
        const trendPercentage = (winningTrades / totalTrades * 100).toFixed(1);
        pnlTrendElement.textContent = `${trendPercentage}%`;

        // Update the class based on positive or negative trend
        if (totalPnl > 0) {
          pnlTrendElement.className = 'trend positive';
        } else {
          pnlTrendElement.className = 'trend negative';
        }
      }

      const totalTradesElement = document.getElementById('total-trades');
      if (totalTradesElement) {
        totalTradesElement.textContent = totalTrades.toLocaleString();
      }

      const winRateElement = document.getElementById('win-rate');
      if (winRateElement) {
        winRateElement.textContent = `${winRate.toFixed(1)}%`;
      }

      // Calculate total fees
      const totalFees = data.performance.response.reduce((sum, item) => {
        return sum + parseFloat(item.fee || 0);
      }, 0);

      // Update total fees element
      const totalFeesElement = document.getElementById('total-fees');
      if (totalFeesElement) {
        totalFeesElement.textContent = `$${totalFees.toFixed(2)}`;
      }

      // Calculate and update fees as percentage of total PnL
      const feesPercentElement = totalFeesElement?.parentElement?.querySelector('.percent');
      if (feesPercentElement && totalPnl !== 0) {
        const feesPercent = Math.abs((totalFees / totalPnl) * 100);
        feesPercentElement.textContent = `${feesPercent.toFixed(2)}%`;
      }

      // Continue with chart update
      convertToPnLData(data.performance.response, start_date, end_date);
      return true;
    } else {
      console.error("Invalid data format received:", data);
      return false;
    }
  } catch (error) {
    console.error('Error updating chart:', error);
    return false;
  }
};

function convertToPnLData(pnl_data, startDate, endDate) {
  console.log("Raw PnL data:", pnl_data);

  // Create a date range from startDate to endDate
  const dateRange = generateDateRange(startDate, endDate);
  console.log("Date range:", dateRange);

  // First, organize data by strategy and date
  const dataByStrategyAndDate = {};

  // Initialize all strategies with 0 PnL for all dates in range
  pnl_data.forEach(data => {
    console.log("Processing data item:", data);
    if (!dataByStrategyAndDate[data.strategy]) {
      dataByStrategyAndDate[data.strategy] = {};
      // Initialize all dates with 0
      dateRange.forEach(date => {
        dataByStrategyAndDate[data.strategy][date] = 0;
      });
    }
  });

  // Sum PnL for each strategy on each date
  pnl_data.forEach(data => {
    // Use end_date instead of exit_date
    const dateToUse = data.end_date || data.exit_date;

    if (dateToUse) {
      // Parse the PnL value directly as a number
      const pnlValue = parseFloat(data.pnl);
      console.log(`Adding PnL ${pnlValue} (original: ${data.pnl}) to strategy ${data.strategy} on ${dateToUse}`);

      // IMPORTANT: Set the value directly instead of adding to existing value
      dataByStrategyAndDate[data.strategy][dateToUse] = pnlValue;
    }
  });

  console.log("Processed data by strategy and date:", dataByStrategyAndDate);

  // Create datasets for Chart.js with cumulative PnL
  const datasets = Object.keys(dataByStrategyAndDate).map(strategy => {
    const strategyData = dataByStrategyAndDate[strategy];

    // Get only dates where this strategy had trades
    const dataPointsByDate = {};
    pnl_data.forEach(item => {
      if (item.strategy === strategy) {
        const dateKey = item.end_date || item.exit_date;
        // Skip invalid dates (1970-01-01 means null/invalid date)
        if (dateKey && dateKey !== '1970-01-01') {
          dataPointsByDate[dateKey] = item;
        }
      }
    });

    // Get sorted list of dates with trades for this strategy
    const tradeDates = Object.keys(dataPointsByDate).sort();

    console.log(`Strategy ${strategy}: Found ${tradeDates.length} trade dates:`, tradeDates);

    // Skip strategies with no trades
    if (tradeDates.length === 0) {
      console.log(`Skipping strategy ${strategy} - no trades`);
      return null;
    }

    // Calculate cumulative PnL only for dates with trades
    let cumulativePnL = 0;
    const chartData = [];
    const originalData = [];

    tradeDates.forEach(date => {
      const dailyPnL = parseFloat(strategyData[date]) || 0;
      cumulativePnL += dailyPnL;
      console.log(`${strategy} - Date: ${date}, Daily PnL: ${dailyPnL}, Cumulative: ${cumulativePnL}`);
      chartData.push({ x: date, y: cumulativePnL });
      originalData.push(dataPointsByDate[date]);
    });

    console.log(`Strategy ${strategy}: Created ${chartData.length} data points`);

    return {
      label: strategy,
      data: chartData,
      fill: false,
      borderColor: getRandomColor(),
      tension: 0.4, // Smooth curves
      pointRadius: 4,
      pointHoverRadius: 6,
      // Store original data for tooltips
      originalData: originalData
    };
  }).filter(dataset => dataset !== null); // Remove null datasets (strategies with no trades)

  // Calculate aggregate PnL - get all unique trade dates across all strategies
  const allTradeDates = new Set();
  pnl_data.forEach(item => {
    const dateKey = item.end_date || item.exit_date;
    // Skip invalid dates (1970-01-01 means null/invalid date)
    if (dateKey && dateKey !== '1970-01-01') {
      allTradeDates.add(dateKey);
    }
  });

  // Sort trade dates
  const sortedTradeDates = Array.from(allTradeDates).sort();

  // Calculate cumulative aggregate PnL only for dates with trades
  let cumulativeAggregate = 0;
  const aggregateChartData = [];

  sortedTradeDates.forEach(date => {
    // Sum all strategies' PnL for this date
    let dailyTotal = 0;
    Object.keys(dataByStrategyAndDate).forEach(strategy => {
      const strategyData = dataByStrategyAndDate[strategy];
      dailyTotal += parseFloat(strategyData[date]) || 0;
    });

    cumulativeAggregate += dailyTotal;
    aggregateChartData.push({ x: date, y: cumulativeAggregate });
  });

  // Add aggregate dataset to the beginning (so it appears first in legend)
  datasets.unshift({
    label: 'agg',
    data: aggregateChartData,
    fill: false,
    borderColor: '#000000', // Black color for aggregate
    borderWidth: 3, // Make it thicker to stand out
    tension: 0.4, // Smooth curves
    pointRadius: 5,
    pointHoverRadius: 7,
    originalData: aggregateChartData.map(() => null)
  });

  console.log('=== FINAL DATASETS ===');
  datasets.forEach(ds => {
    console.log(`Dataset: ${ds.label}, Points: ${ds.data.length}, First: ${JSON.stringify(ds.data[0])}, Last: ${JSON.stringify(ds.data[ds.data.length - 1])}`);
  });

  // Update the chart with the new datasets
  if (myLineChart) {
    // Check for negative values in the datasets
    datasets.forEach(dataset => {
      const negativeValues = dataset.data.filter(point => point.y < 0);
      console.log(`Dataset ${dataset.label} has ${negativeValues.length} negative values`);
      if (negativeValues.length > 0) {
        console.log("Sample negative values:", negativeValues.slice(0, 3));
      }
    });

    myLineChart.data.datasets = datasets;
    myLineChart.update();
    console.log("Chart updated with datasets:", datasets);
  } else {
    console.error("Chart not initialized");
  }
}

// Helper function to generate an array of dates between start and end
function generateDateRange(startDate, endDate) {
  const dates = [];
  let currentDate = new Date(startDate);
  const end = new Date(endDate);

  while (currentDate <= end) {
    dates.push(currentDate.toISOString().split('T')[0]);
    currentDate.setDate(currentDate.getDate() + 1);
  }

  return dates;
}

function getRandomColor() {
  const letters = '0123456789ABCDEF';
  let color = '#';
  for (let i = 0; i < 6; i++) {
    color += letters[Math.floor(Math.random() * 16)];
  }
  return color;
}

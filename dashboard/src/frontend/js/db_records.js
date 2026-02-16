// db_records.js

// Function to fetch strategy data
function formatLocal(ts) {
  try {
    if (!ts) return '';
    // Parse as Date, render in LOCAL time and append local TZ short name
    const d = new Date(ts);
    if (!isNaN(d)) {
      const pad = (n) => String(n).padStart(2, '0');
      const y = d.getFullYear();
      const mo = pad(d.getMonth() + 1);
      const da = pad(d.getDate());
      const hh = pad(d.getHours());
      const mm = pad(d.getMinutes());
      const ss = pad(d.getSeconds());
      // Get local timezone short name using Intl
      let tz = '';
      const iana = (Intl.DateTimeFormat().resolvedOptions().timeZone) || undefined;
      try {
        const dtf = new Intl.DateTimeFormat(undefined, { timeZone: iana, timeZoneName: 'short' });
        const parts = dtf.formatToParts(d);
        const tzPart = parts.find(p => p.type === 'timeZoneName');
        let label = tzPart ? tzPart.value : '';
        // Special-case Europe/London to show BST/GMT explicitly
        if (iana === 'Europe/London') {
          const jan = new Date(d.getFullYear(), 0, 1);
          const jul = new Date(d.getFullYear(), 6, 1);
          const stdOffset = Math.max(jan.getTimezoneOffset(), jul.getTimezoneOffset());
          const isDst = d.getTimezoneOffset() < stdOffset;
          label = isDst ? 'BST' : 'GMT';
        }
        tz = label || '';
      } catch (_) { }
      const spacing = '\u00A0\u00A0';
      const datePart = `${y}-${mo}-${da}`;
      const timePart = `${hh}:${mm}:${ss}`;
      const tzPartOut = tz ? `${spacing}${tz}` : '';
      return `${datePart}${spacing}${timePart}${tzPartOut}`;
    }
  } catch (_) { }
  return String(ts);
}
// Simplified fetchStrategyData function with better error handling
// Function to flatten JSON fields in a strategy record
async function fetchStrategyData(symbol = null, fromDate = null, toDate = null) {
  try {
    // Determine the URL based on whether a symbol is provided
    const baseUrl = symbol ? `/strategy/${symbol}` : '/universe';

    let fromStr, toStr;

    if (fromDate && toDate) {
      fromStr = fromDate;
      toStr = toDate;
    } else {
      // Create default date range (last 365 days to today)
      const today = new Date();
      const pastDate = new Date();
      pastDate.setDate(today.getDate() - 365);

      // Format dates as ISO strings (YYYY-MM-DD)
      fromStr = pastDate.toISOString().split('T')[0];
      toStr = today.toISOString().split('T')[0];
    }

    // Build URL with required date parameters
    const url = `${baseUrl}?from=${fromStr}&to=${toStr}`;

    console.log(`Fetching data from: ${url}`);

    // Make the request with explicit options
    const response = await fetch(url, {
      method: 'GET',
      headers: {
        'Accept': 'application/json'
      }
    });

    // Log response status
    console.log(`Response status: ${response.status}`);

    // Check if response is OK
    if (!response.ok) {
      // Try to get error details from response
      let errorDetails = '';
      try {
        const errorText = await response.text();
        errorDetails = errorText;
      } catch (e) {
        errorDetails = 'Could not extract error details';
      }

      throw new Error(`HTTP error! Status: ${response.status}, Details: ${errorDetails}`);
    }

    // Parse JSON response and normalize to array
    const raw = await response.json();
    let data = [];
    if (Array.isArray(raw)) {
      data = raw;
    } else if (raw && raw.strategies && Array.isArray(raw.strategies.response)) {
      data = raw.strategies.response;
    } else if (raw && Array.isArray(raw.response)) {
      data = raw.response;
    }

    console.log(`Successfully fetched ${data.length} records`);
    return data;
  } catch (error) {
    console.error('Error in fetchStrategyData:', error);
    // Return empty array but also throw the error so caller can handle it
    return [];
  }
}

function flattenRecord(record) {
  const flatRecord = { ...record };

  // Flatten risk data if it exists
  if (record.risk && typeof record.risk === 'object') {
    // Handle risk.gain
    if (record.risk.gain && typeof record.risk.gain === 'object') {
      Object.entries(record.risk.gain).forEach(([key, value]) => {
        flatRecord[`risk_gain_${key}`] = (value !== null && typeof value === 'object') ? JSON.stringify(value) : value;
      });
    }

    // Handle risk.loss
    if (record.risk.loss && typeof record.risk.loss === 'object') {
      Object.entries(record.risk.loss).forEach(([key, value]) => {
        flatRecord[`risk_loss_${key}`] = (value !== null && typeof value === 'object') ? JSON.stringify(value) : value;
      });
    }

    // Handle risk.stats
    if (record.risk.stats && typeof record.risk.stats === 'object') {
      Object.entries(record.risk.stats).forEach(([key, value]) => {
        flatRecord[`risk_stats_${key}`] = value;
      });
    }

    // Handle other top-level risk properties
    Object.entries(record.risk).forEach(([key, value]) => {
      if (!['gain', 'loss', 'stats'].includes(key)) {
        flatRecord[`risk_${key}`] = (value !== null && typeof value === 'object') ? JSON.stringify(value) : value;
      }
    });
  }

  // Flatten metadata if it exists
  if (record.meta && typeof record.meta === 'object') {
    Object.entries(record.meta).forEach(([key, value]) => {
      flatRecord[`meta_${key}`] = (value !== null && typeof value === 'object') ? JSON.stringify(value) : value;
    });
  }

  // Calculate unrealized P&L for open positions and set entry price
  if (record.status === 1 || record.status === "Open") {
    const entry = parseFloat(flatRecord.risk_gain_open) || 0;
    const current = parseFloat(flatRecord.risk_gain_current) || 0;
    const priceEffect = flatRecord.meta_price_effect;

    if (entry > 0 && current > 0 && priceEffect) {
      let unrealizedPnl = 0;
      if (priceEffect === "Credit") {
        // For credit spreads: profit when price decreases
        unrealizedPnl = entry - current;
      } else {
        // For debit spreads: profit when price increases
        unrealizedPnl = current - entry;
      }
      flatRecord.risk_stats_pnl = unrealizedPnl.toFixed(2);
    }

    // For open trades, ensure entry price is displayed
    flatRecord.risk_gain_open = flatRecord.risk_gain_open || flatRecord.meta_open_price;
  } else {
    // For closed trades, use meta_open_price as the entry price
    flatRecord.risk_gain_open = flatRecord.meta_open_price || flatRecord.risk_gain_open;
  }

  // Remove the original nested objects to avoid duplication
  delete flatRecord.risk;
  delete flatRecord.meta;
  delete flatRecord.account;

  return flatRecord;
}

// Function to render the strategy table using DataTables
// Function to render the strategy table using DataTables
async function renderStrategyTable(containerId, symbol = null, fromDate = null, toDate = null) {
  try {
    // Check for period select and use it if no specific dates provided
    const periodSelect = document.getElementById('period-select');
    if (periodSelect) {
      // If no dates provided, calculate from dropdown
      if (!fromDate || !toDate) {
        const months = parseInt(periodSelect.value || "12"); // Default to 12 months if value missing
        const today = new Date();
        const start = new Date();
        start.setMonth(today.getMonth() - months);

        fromDate = start.toISOString().split('T')[0];
        toDate = today.toISOString().split('T')[0];
      }

      // Bind change event (using jQuery off/on to prevent duplicate listeners)
      $(periodSelect).off('change').on('change', function () {
        const months = parseInt(this.value);
        const today = new Date();
        const start = new Date();
        start.setMonth(today.getMonth() - months);

        const newFrom = start.toISOString().split('T')[0];
        const newTo = today.toISOString().split('T')[0];

        // Reload table with new dates
        renderStrategyTable(containerId, symbol, newFrom, newTo);
      });
    }

    // Fetch the data
    const data = await fetchStrategyData(symbol, fromDate, toDate);

    if (!data || data.length === 0) {
      document.getElementById(containerId).innerHTML = '<p>No data available</p>';
      return;
    }

    // Flatten all records
    const flattenedData = data.map(flattenRecord);

    // IMPORTANT: Destroy existing table if it exists
    if ($.fn.DataTable.isDataTable('#strategy-table')) {
      $('#strategy-table').DataTable().destroy();
      $('#strategy-table').empty();
    }

    // Create a fresh table element
    $('#' + containerId).html('<table id="strategy-table" class="display" style="width:100%"></table>');


    // Add window resize handler to keep table responsive
    $(window).on('resize', function () {
      table.columns.adjust().draw();
    });

    // Debug approach - create an array of objects with explicit properties
    const tableData = flattenedData.map(record => {
      return {
        symbol: record.symbol || '',
        status: record.status || '',
        meta_type: record.meta_type || '',
        risk_side: record.risk_side || '',
        meta_quantity: record.meta_quantity || '',
        entry_time: record.entry_time || '',
        exit_time: record.exit_time || '',
        risk_gain_open: record.risk_gain_open || '',
        risk_gain_target: record.risk_gain_target || '',
        risk_gain_current: record.risk_gain_current || '',
        risk_loss_target: record.risk_loss_target || '',
        risk_loss_watermark: record.risk_loss_watermark || '',
        risk_stats_pnl: record.risk_stats_pnl || '',
        risk_stats_roi: record.risk_stats_roi || '',
        risk_stats_fee: record.risk_stats_fee || ''
      };
    });

    // Log the first record to verify structure
    console.log("Prepared table data first record:", tableData[0]);
    const originalData = [...tableData];


    // Initialize DataTable with explicit columns and data
    const table = $('#strategy-table').DataTable({
      data: tableData,
      columns: [
        { title: "Symbol", data: "symbol" },
        { title: "Status", data: "status" },
        { title: "Type", data: "meta_type" },
        { title: "Side", data: "risk_side" },
        { title: "Qty", data: "meta_quantity", className: 'all' },
        { title: "Entry Time", data: "entry_time", render: (data) => formatLocal(data) },
        { title: "Exit Time", data: "exit_time", render: (data) => formatLocal(data) },
        { title: "Entry", data: "risk_gain_open", render: (data) => data || "0.00" },
        { title: "Mark", data: "risk_gain_current" },
        { title: "Loss Target", data: "risk_loss_target" },
        { title: "Watermark", data: "risk_loss_watermark" },
        {
          title: "PnL", data: "risk_stats_pnl", render: (data, type, row) => {
            const pnl = parseFloat(data) || 0;
            const color = pnl >= 0 ? 'green' : 'red';
            return `<span style="color: ${color}">${data}</span>`;
          }
        },
        { title: "ROI", data: "risk_stats_roi" },
        { title: "Fees", data: "risk_stats_fee" }
      ],
      order: [[4, 'desc']], // Sort by entry_time
      pageLength: 10,
      // Disable built-in search
      searching: false
    });

    // Remove the default search box
    $('.dataTables_filter').remove();

    // Clean up any existing length control in our custom toolbar (from previous renders)
    $('.date-controls .dataTables_length').remove();

    // Move the new DataTables length control to our custom toolbar (after period selector)
    const lengthControl = $('.dataTables_length');
    // Add some margin for spacing
    lengthControl.css({
      'margin-left': '20px',
      'margin-right': 'auto' // Push search box to the right if using flex
    });
    lengthControl.insertAfter('.date-control');

    // Add our own search box to the date-controls container if it doesn't exist
    if ($('#custom-search-input').length === 0) {
      $('.date-controls').append(`
        <div class="custom-search">
          <label>Search: <input type="text" id="custom-search-input" style="padding: 5px; border: 1px solid #ccc;"></label>
        </div>
      `);
    }

    // Implement our own search function - unbind first to avoid duplicates/stale closures
    $('#custom-search-input').off('keyup').on('keyup', function () {
      const searchTerm = this.value.toLowerCase();

      if (!searchTerm) {
        // If search is empty, restore original data
        table.clear().rows.add(originalData).draw();
        return;
      }

      // Filter the data manually
      const filteredData = originalData.filter(row => {
        return Object.values(row).some(value => {
          // Convert value to string and check if it contains the search term
          return String(value).toLowerCase().includes(searchTerm);
        });
      });

      // Update the table with filtered data
      table.clear().rows.add(filteredData).draw();

      console.log(`Search "${searchTerm}" found ${filteredData.length} matches`);
    });

    // // Initialize date pickers
    // const dateFormat = 'YYYY-MM-DD';
    // let minDate = moment().subtract(30, 'days');
    // let maxDate = moment();

    // // Setup date range inputs
    // $('#min').val(minDate.format(dateFormat));
    // $('#max').val(maxDate.format(dateFormat));



    // // Custom filtering function for date range
    // $.fn.dataTable.ext.search.push(
    //   function (settings, data, dataIndex) {
    //     // Find date columns
    //     const dateColumnIndex = Object.keys(flattenedData[0]).findIndex(key =>
    //       key.includes('time') || key.includes('date')
    //     );

    //     if (dateColumnIndex < 0) return true;

    //     const min = moment($('#min').val(), dateFormat);
    //     const max = moment($('#max').val(), dateFormat);
    //     const date = moment(data[dateColumnIndex]);

    //     return (
    //       (min.isSame(moment('0000-00-00')) || min.isSameOrBefore(date)) &&
    //       (max.isSame(moment('0000-00-00')) || max.isSameOrAfter(date))
    //     );
    //   }
    // );
  } catch (error) {
    console.error('Error in renderStrategyTable:', error);
    document.getElementById(containerId).innerHTML = '<p>Error loading data</p>';
  }
}

// Refilter the table when date inputs
// Export functions for use in other files
export { renderStrategyTable, fetchStrategyData };
// db_records.js

// Function to fetch strategy data
// Simplified fetchStrategyData function with better error handling
// Function to flatten JSON fields in a strategy record
async function fetchStrategyData(symbol = null) {
  try {
    // Determine the URL based on whether a symbol is provided
    const baseUrl = symbol ? `/strategy/${symbol}` : '/universe';

    // Create default date range (last 90 days to today)
    const today = new Date();
    const ninetyDaysAgo = new Date();
    ninetyDaysAgo.setDate(today.getDate() - 90);

    // Format dates as ISO strings (YYYY-MM-DD)
    const fromDate = ninetyDaysAgo.toISOString().split('T')[0];
    const toDate = today.toISOString().split('T')[0];

    // Build URL with required date parameters
    const url = `${baseUrl}?from=${fromDate}&to=${toDate}`;

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
        flatRecord[`risk_gain_${key}`] = typeof value === 'object' ? JSON.stringify(value) : value;
      });
    }

    // Handle risk.loss
    if (record.risk.loss && typeof record.risk.loss === 'object') {
      Object.entries(record.risk.loss).forEach(([key, value]) => {
        flatRecord[`risk_loss_${key}`] = typeof value === 'object' ? JSON.stringify(value) : value;
      });
    }

    // Handle risk.stats
    if (record.risk.stats && typeof record.risk.stats === 'object') {
      Object.entries(record.risk.stats).forEach(([key, value]) => {
        flatRecord[`risk_stats_${key}`] = typeof value === 'object' ? JSON.stringify(value) : value;
      });
    }

    // Handle other top-level risk properties
    Object.entries(record.risk).forEach(([key, value]) => {
      if (!['gain', 'loss', 'stats'].includes(key)) {
        flatRecord[`risk_${key}`] = typeof value === 'object' ? JSON.stringify(value) : value;
      }
    });
  }

  // Flatten metadata if it exists
  if (record.meta && typeof record.meta === 'object') {
    Object.entries(record.meta).forEach(([key, value]) => {
      flatRecord[`meta_${key}`] = typeof value === 'object' ? JSON.stringify(value) : value;
    });
    console.log('flatRecord:', flatRecord);
  }

  // Remove the original nested objects to avoid duplication
  delete flatRecord.risk;
  delete flatRecord.meta;
  delete flatRecord.account;

  return flatRecord;
}

// Function to render the strategy table using DataTables
// Function to render the strategy table using DataTables
async function renderStrategyTable(containerId, symbol = null) {
  try {
    // Fetch the data
    const data = await fetchStrategyData(symbol);

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
        entry_time: record.entry_time || '',
        exit_time: record.exit_time || '',
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
        { title: "Entry Time", data: "entry_time" },
        { title: "Exit Time", data: "exit_time" },
        { title: "Profit Target", data: "risk_gain_target" },
        { title: "Mark", data: "risk_gain_current" },
        { title: "Loss Target", data: "risk_loss_target" },
        { title: "Watermark", data: "risk_loss_watermark" },
        { title: "PnL", data: "risk_stats_pnl" },
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

    // Add our own search box
    $('#data-container').prepend(`
      <div class="custom-search" style="text-align: right; margin-bottom: 10px;">
        <label>Search: <input type="text" id="custom-search-input" style="padding: 5px; border: 1px solid #ccc;"></label>
      </div>
    `);

    // Implement our own search function
    $('#custom-search-input').on('keyup', function () {
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
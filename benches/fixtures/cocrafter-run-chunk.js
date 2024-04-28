  /*
  * Copyright Bubble Group, Inc.
  */

  window.bubble_bundle_name = 'run';

  if (!window._bubble_page_load_data) {
      window._bubble_page_load_data = {};
  }
  if (!window._bubble_page_load_data.js_execution_timings) {
    window._bubble_page_load_data.js_execution_timings = {};
  }
  if (!window._bubble_page_load_data.js_execution_timings.module_execution_timings) {
    window._bubble_page_load_data.js_execution_timings.module_execution_timings = [];
  }
  window._bubble_page_load_data.js_execution_timings.module_start_execution = Date.now();
  if (!window._bubble_page_load_data.js_execution_timings.module_first_execution) {
    window._bubble_page_load_data.js_execution_timings.module_first_execution = window._bubble_page_load_data.js_execution_timings.module_start_execution;
  }
/*! VelocityJS.org (1.4.0). (C) 2014 Julian Shapiro. MIT @license: en.wikipedia.org/wiki/MIT_License */
/*! VelocityJS.org jQuery Shim (1.0.1). (C) 2014 The jQuery Foundation. MIT @license: en.wikipedia.org/wiki/MIT_License. */
/* NProgress, (c) 2013, 2014 Rico Sta. Cruz - http://ricostacruz.com/nprogress
 * @license MIT */
/*! Bundled license information:

buffer/index.js:
  (*!
   * The buffer module from node.js, for the browser.
   *
   * @author   Feross Aboukhadijeh <https://feross.org>
   * @license  MIT
   *)

moment/moment.js:
  (*! moment.js *)
  (*! version : 2.30.1 *)
  (*! authors : Tim Wood, Iskren Chernev, Moment.js contributors *)
  (*! license : MIT *)
  (*! momentjs.com *)

punycode/punycode.js:
  (*! https://mths.be/punycode v1.3.2 by @mathias *)

lodash-es/lodash.js:
  (**
   * @license
   * Lodash (Custom Build) <https://lodash.com/>
   * Build: `lodash modularize exports="es" -o ./`
   * Copyright OpenJS Foundation and other contributors <https://openjsf.org/>
   * Released under MIT license <https://lodash.com/license>
   * Based on Underscore.js 1.8.3 <http://underscorejs.org/LICENSE>
   * Copyright Jeremy Ashkenas, DocumentCloud and Investigative Reporters & Editors
   *)

@noble/ciphers/esm/utils.js:
  (*! noble-ciphers - MIT License (c) 2023 Paul Miller (paulmillr.com) *)

@noble/hashes/esm/utils.js:
  (*! noble-hashes - MIT License (c) 2022 Paul Miller (paulmillr.com) *)
*/
window._bubble_page_load_data.js_execution_timings.module_execution_timings.push(Date.now()-window._bubble_page_load_data.js_execution_timings.module_start_execution);
// @ts-ignore
import { capi } from "./build-no-tables-utf32.js";
export { capi };
import TABLE_DATA_1 from "./en-ueb-g1.ctb?raw";
import TABLE_DATA_2 from "./en-ueb-g2.ctb?raw";
import TABLE_DATA_3 from "./en-ueb-chardefs.uti?raw";
import TABLE_DATA_4 from "./en-ueb-math.ctb?raw";
import TABLE_DATA_5 from "./text_nabcc.dis?raw";
import TABLE_DATA_6 from "./spaces.uti?raw";
import TABLE_DATA_7 from "./latinLetterDef6Dots.uti?raw";
import TABLE_DATA_8 from "./latinUppercaseComp6.uti?raw";
import TABLE_DATA_9 from "./braille-patterns.cti?raw";
const TABLE_NAME_1 = "en-ueb-g1.ctb";
const TABLE_NAME_2 = "en-ueb-g2.ctb";
const TABLE_NAME_3 = "en-ueb-chardefs.uti";
const TABLE_NAME_4 = "en-ueb-math.ctb";
const TABLE_NAME_5 = "text_nabcc.dis";
const TABLE_NAME_6 = "spaces.uti";
const TABLE_NAME_7 = "latinLetterDef6Dots.uti";
const TABLE_NAME_8 = "latinUppercaseComp6.uti";
const TABLE_NAME_9 = "braille-patterns.cti";
const TABLE_FOLDER = "/usr/local/share/liblouis/tables";

let tablesInitialized = false;
export function initTables() {
    if (tablesInitialized) {
        return;
    }
    capi.FS.mkdirTree(TABLE_FOLDER);
    const tables = [
        [TABLE_NAME_1, TABLE_DATA_1],
        [TABLE_NAME_2, TABLE_DATA_2],
        [TABLE_NAME_3, TABLE_DATA_3],
        [TABLE_NAME_4, TABLE_DATA_4],
        [TABLE_NAME_5, TABLE_DATA_5],
        [TABLE_NAME_6, TABLE_DATA_6],
        [TABLE_NAME_7, TABLE_DATA_7],
        [TABLE_NAME_8, TABLE_DATA_8],
        [TABLE_NAME_9, TABLE_DATA_9],
    ];
    for (const [name, data] of tables) {
        capi.FS.writeFile(TABLE_FOLDER + "/" + name, data);
    }
    tablesInitialized = true;
}

export function version() {
    return capi.ccall("lou_version", "string", [], []);
}

export function setLogLevel(num: number) {
    return capi.ccall("lou_setLogLevel", "void", ["number"], [num]);
}

export function getTable(str: string) {
    return capi.ccall("lou_getTable", "number", ["string"], [str]);
}

export function checkTable(str: string) {
    return capi.ccall("lou_checkTable", "number", ["string"], [str]);
}

export function free() {
    return capi.ccall("lou_free", "void", [], []);
}

export function charSize() {
    return capi.ccall("lou_charSize", "number", [], []);
}

export function logFile(str: string) {
    return capi.ccall("lou_logFile", "void", ["string"], [str]);
}

export function getFilesystem() {
    return capi.FS;
}

let currentLogFunctionPointer = 0;
/**
 * Register a function that is called when liblouis logs something.
 * The function should have the signature `function (logLvl: number, msg: string)`.
 * If no function is provided, the default log function will be used.
 */
export function registerLogCallback(fn?: Function) {
    if (currentLogFunctionPointer) {
        capi.Runtime.removeFunction(currentLogFunctionPointer);
    }

    currentLogFunctionPointer = capi.Runtime.addFunction(function (
        logLvl: LogLevel,
        msg: string,
    ) {
        (fn || easyApiDefaultLogCallback)(logLvl, capi.Pointer_stringify(msg));
    });

    capi.ccall(
        "lou_registerLogCallback",
        "void",
        ["pointer"],
        [currentLogFunctionPointer],
    );
}

export function compileString(table: string, str: string) {
    var success = capi.ccall(
        "lou_compileString",
        "number",
        ["string", "string"],
        [table, str],
    );
    return !!success;
}

export const mem = {
    getBufferLength: function (buff: string) {
        return (buff.length + 1) * 4;
    },

    transfer: function (buff: string) {
        var bufflen = mem.getBufferLength(buff);
        var buff_ptr = capi._malloc(bufflen);

        capi.stringToUTF32(buff, buff_ptr, bufflen);

        return buff_ptr;
    },

    read: function (buffptr: number, bufflenptr: number) {
        if (!bufflenptr) {
            // null-terminated string
            return capi.UTF32ToString(buffptr);
        } else {
            var start_index = buffptr >> 2;
            var end_index = start_index + capi.getValue(bufflenptr, "i32");
            var outstr_buff = capi.HEAPU32.slice(start_index, end_index);
            return outstr_buff;
        }
    },

    buffToString: function (buff: Int32Array) {
        var str = "";

        for (var i = 0; i < buff.length; ++i) {
            var utf32 = buff[i];
            // taken from emscripten, which ported it from
            // http://unicode.org/faq/utf_bom.html#utf16-3
            if (utf32 >= 0x10000) {
                var ch = utf32 - 0x10000;
                str += String.fromCharCode(
                    0xd800 | (ch >> 10),
                    0xdc00 | (ch & 0x3ff),
                );
            } else {
                str += String.fromCharCode(utf32);
            }
        }

        return str;
    },
};

/**
 * Translate a string to Braille using Liblouis. Currently only two tables are supported (they are hardcoded):
 * - `"en-ueb-g2.ctb"`
 * - `"en-ueb-g1.ctb"`
 */
export function translateString(
    table: "en-ueb-g2.ctb" | "en-ueb-g1.ctb",
    inbuf: string,
    backtranslate: boolean = false,
) {
    if (typeof inbuf !== "string" || inbuf.length === 0) {
        return "";
    }
    if (!tablesInitialized) {
        initTables();
    }

    var mode = 0;

    var inbuff_ptr = mem.transfer(inbuf);
    var bufflen = mem.getBufferLength(inbuf);
    var outbuff_ptr = capi._malloc(bufflen);

    // in emscripten we need a 32bit cell for each pointer
    var bufflen_ptr = capi._malloc(4);
    var strlen_ptr = capi._malloc(4);

    capi.setValue(bufflen_ptr, bufflen, "i32");
    capi.setValue(strlen_ptr, bufflen, "i32");

    var success = capi.ccall(
        backtranslate ? "lou_backTranslateString" : "lou_translateString",
        "number",
        ["string", "number", "number", "number", "number", "number", "number"],
        [
            table,
            inbuff_ptr,
            strlen_ptr,
            outbuff_ptr,
            bufflen_ptr,
            null,
            null,
            mode,
        ],
    );

    if (!success) {
        return null;
    }

    var outbuff = mem.read(outbuff_ptr, bufflen_ptr);

    // XXX: (2024-12-14) I don't know why freeing these causes an error. When this operation fails, there is probably a memory leak.
    try {
        capi._free(outbuff_ptr);
        capi._free(inbuff_ptr);
        capi._free(bufflen_ptr);
        capi._free(strlen_ptr);
    } catch (e) {
        console.warn("Failed to free memory during Braille translation");
    }

    return mem.buffToString(outbuff);
}

const _CONSOLE_MAPPING = {
    ALL: "log",
    DEBUG: "log",
    INFO: "info",
    WARN: "warn",
    ERROR: "error",
    FATAL: "error",
};

const LOG = {
    ALL: 0,
    DEBUG: 10000,
    INFO: 20000,
    WARN: 30000,
    ERROR: 40000,
    FATAL: 50000,
    OFF: 60000,
    0: "ALL",
    10000: "DEBUG",
    20000: "INFO",
    30000: "WARN",
    40000: "ERROR",
    50000: "FATAL",
    60000: "OFF",
};
type LogLevel = keyof typeof LOG & number;

function easyApiDefaultLogCallback(lvl_id: LogLevel, msg: string) {
    var lvl_name = LOG[lvl_id];
    msg = "[" + lvl_name + "] " + msg;

    (
        (console as any)[
            _CONSOLE_MAPPING[lvl_name as keyof typeof _CONSOLE_MAPPING]
        ] || console.log
    )(msg);
}

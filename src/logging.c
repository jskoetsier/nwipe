/*
 *  logging.c:  Logging facilities for nwipe.
 *
 *  Copyright Darik Horn <dajhorn-dban@vanadac.com>.
 *
 *  This program is free software; you can redistribute it and/or modify it under
 *  the terms of the GNU General Public License as published by the Free Software
 *  Foundation, version 2.
 *
 *  This program is distributed in the hope that it will be useful, but WITHOUT
 *  ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS
 *  FOR A PARTICULAR PURPOSE.  See the GNU General Public License for more
 *  details.
 *
 *  You should have received a copy of the GNU General Public License along with
 *  this program; if not, write to the Free Software Foundation, Inc.,
 *  51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
 *
 */

#ifndef _DEFAULT_SOURCE
#define _DEFAULT_SOURCE
#endif

#ifndef _POSIX_SOURCE
#define _POSIX_SOURCE
#endif

#include "stdio.h"
#include "stdlib.h"
#include "string.h"
#include "stdarg.h"
#include "nwipe.h"
#include "context.h"
#include "method.h"
#include "prng.h"
#include "options.h"
#include "logging.h"

/* Global array to hold log values to print when logging to STDOUT */
char** log_lines;
int log_current_element = 0;
int log_elements_allocated = 0;
int log_elements_displayed = 0;
pthread_mutex_t mutex1 = PTHREAD_MUTEX_INITIALIZER;

void nwipe_log( nwipe_log_t level, const char* format, ... )
{
    /**
     *  Writes a message to the program log file.
     *
     */

    char** result;
    char* malloc_result;
    char message_buffer[MAX_LOG_LINE_CHARS * sizeof( char )];
    int chars_written;

    int message_buffer_length;
    int r; /* result buffer */

    /* A time buffer. */
    time_t t;

    /* A pointer to the system time struct. */
    struct tm* p;
    r = pthread_mutex_lock( &mutex1 );
    if( r != 0 )
    {
        fprintf( stderr, "nwipe_log: pthread_mutex_lock failed. Code %i \n", r );
        return;
    }

    /* Get the current time. */
    t = time( NULL );
    p = localtime( &t );

    /* Position of writing to current log string */
    int line_current_pos = 0;

    /* initialise characters written */
    chars_written = 0;

    /* Print the date. The rc script uses the same format. */
    if( level != NWIPE_LOG_NOTIMESTAMP )
    {
        chars_written = snprintf( message_buffer,
                                  MAX_LOG_LINE_CHARS,
                                  "[%i/%02i/%02i %02i:%02i:%02i] ",
                                  1900 + p->tm_year,
                                  1 + p->tm_mon,
                                  p->tm_mday,
                                  p->tm_hour,
                                  p->tm_min,
                                  p->tm_sec );
    }

    /*
     * Has the end of the buffer been reached ?, snprintf returns the number of characters that would have been
     * written if MAX_LOG_LINE_CHARS had not been reached, it does not return the actual characters written in
     * all circumstances, hence why we need to check whether it's greater than MAX_LOG_LINE_CHARS and if so set
     * it to MAX_LOG_LINE_CHARS, preventing a buffer overrun further down this function.
     */

    /* check if there was a complete failure to write this part of the message, in which case return */
    if( chars_written < 0 )
    {
        fprintf( stderr, "nwipe_log: snprintf error when writing log line to memory.\n" );
        r = pthread_mutex_unlock( &mutex1 );
        if( r != 0 )
        {
            fprintf( stderr, "nwipe_log: pthread_mutex_unlock failed. Code %i \n", r );
            return;
        }
    }
    else
    {
        if( ( line_current_pos + chars_written ) > MAX_LOG_LINE_CHARS )
        {
            fprintf( stderr,
                     "nwipe_log: Warning! The log line has been truncated as it exceeded %i characters\n",
                     MAX_LOG_LINE_CHARS );
            line_current_pos = MAX_LOG_LINE_CHARS;
        }
        else
        {
            line_current_pos += chars_written;
        }
    }

    if( line_current_pos < MAX_LOG_LINE_CHARS )
    {
        switch( level )
        {

            case NWIPE_LOG_NONE:
            case NWIPE_LOG_NOTIMESTAMP:
                /* Do nothing. */
                break;

            case NWIPE_LOG_DEBUG:
                chars_written =
                    snprintf( message_buffer + line_current_pos, MAX_LOG_LINE_CHARS - line_current_pos, "debug: " );
                break;

            case NWIPE_LOG_INFO:
                chars_written =
                    snprintf( message_buffer + line_current_pos, MAX_LOG_LINE_CHARS - line_current_pos, "info: " );
                break;

            case NWIPE_LOG_NOTICE:
                chars_written =
                    snprintf( message_buffer + line_current_pos, MAX_LOG_LINE_CHARS - line_current_pos, "notice: " );
                break;

            case NWIPE_LOG_WARNING:
                chars_written =
                    snprintf( message_buffer + line_current_pos, MAX_LOG_LINE_CHARS - line_current_pos, "warning: " );
                break;

            case NWIPE_LOG_ERROR:
                chars_written =
                    snprintf( message_buffer + line_current_pos, MAX_LOG_LINE_CHARS - line_current_pos, "error: " );
                break;

            case NWIPE_LOG_FATAL:
                chars_written =
                    snprintf( message_buffer + line_current_pos, MAX_LOG_LINE_CHARS - line_current_pos, "fatal: " );
                break;

            case NWIPE_LOG_SANITY:
                /* TODO: Request that the user report the log. */
                chars_written =
                    snprintf( message_buffer + line_current_pos, MAX_LOG_LINE_CHARS - line_current_pos, "sanity: " );
                break;

            default:
                chars_written = snprintf(
                    message_buffer + line_current_pos, MAX_LOG_LINE_CHARS - line_current_pos, "level %i: ", level );
        }

        /*
         * Has the end of the buffer been reached ?
         */
        if( chars_written < 0 )
        {
            fprintf( stderr, "nwipe_log: snprintf error when writing log line to memory.\n" );
            r = pthread_mutex_unlock( &mutex1 );
            if( r != 0 )
            {
                fprintf( stderr, "nwipe_log: pthread_mutex_unlock failed. Code %i \n", r );
                return;
            }
        }
        else
        {
            if( ( line_current_pos + chars_written ) > MAX_LOG_LINE_CHARS )
            {
                fprintf( stderr,
                         "nwipe_log: Warning! The log line has been truncated as it exceeded %i characters\n",
                         MAX_LOG_LINE_CHARS );
                line_current_pos = MAX_LOG_LINE_CHARS;
            }
            else
            {
                line_current_pos += chars_written;
            }
        }
    }

    /* The variable argument pointer. */
    va_list ap;

    /* Fetch the argument list. */
    va_start( ap, format );

    /* Print the event. */
    if( line_current_pos < MAX_LOG_LINE_CHARS )
    {
        chars_written =
            vsnprintf( message_buffer + line_current_pos, MAX_LOG_LINE_CHARS - line_current_pos - 1, format, ap );

        if( chars_written < 0 )
        {
            fprintf( stderr, "nwipe_log: snprintf error when writing log line to memory.\n" );
            r = pthread_mutex_unlock( &mutex1 );
            if( r != 0 )
            {
                fprintf( stderr, "nwipe_log: pthread_mutex_unlock failed. Code %i \n", r );
                va_end( ap );
                return;
            }
        }
        else
        {
            if( ( line_current_pos + chars_written ) > MAX_LOG_LINE_CHARS )
            {
                fprintf( stderr,
                         "nwipe_log: Warning! The log line has been truncated as it exceeded %i characters\n",
                         MAX_LOG_LINE_CHARS );
                line_current_pos = MAX_LOG_LINE_CHARS;
            }
            else
            {
                line_current_pos += chars_written;
            }
        }
    }

    fflush( stdout );
    /* Increase the current log element pointer - we will write here, deallocation is done in cleanup() in nwipe.c */
    if( log_current_element == log_elements_allocated )
    {
        log_elements_allocated++;
        result = realloc( log_lines, ( log_elements_allocated ) * sizeof( char* ) );
        if( result == NULL )
        {
            fprintf( stderr, "nwipe_log: realloc failed when adding a log line.\n" );
            r = pthread_mutex_unlock( &mutex1 );
            if( r != 0 )
            {
                fprintf( stderr, "nwipe_log: pthread_mutex_unlock failed. Code %i \n", r );
                va_end( ap );
                return;
            }
        }
        log_lines = result;

        /* Allocate memory for storing a single log message, deallocation is done in cleanup() in nwipe.c */
        message_buffer_length = strlen( message_buffer ) * sizeof( char );
        malloc_result = malloc( ( message_buffer_length + 1 ) * sizeof( char ) );
        if( malloc_result == NULL )
        {
            fprintf( stderr, "nwipe_log: malloc failed when adding a log line.\n" );
            r = pthread_mutex_unlock( &mutex1 );
            if( r != 0 )
            {
                fprintf( stderr, "nwipe_log: pthread_mutex_unlock failed. Code %i \n", r );
                va_end( ap );
                return;
            }
        }
        log_lines[log_current_element] = malloc_result;
    }

    strcpy( log_lines[log_current_element], message_buffer );

    /*
        if( level >= NWIPE_LOG_WARNING )
        {
            vfprintf( stderr, format, ap );
        }
    */

    /* Release the argument list. */
    va_end( ap );

    /*
        if( level >= NWIPE_LOG_WARNING )
        {
            fprintf( stderr, "\n" );
        }
    */

    /* The log file pointer. */
    FILE* fp;

    /* The log file descriptor. */
    int fd;

    if( nwipe_options.logfile[0] == '\0' )
    {
        if( nwipe_options.nogui )
        {
            printf( "%s\n", log_lines[log_current_element] );
            log_elements_displayed++;
        }
    }
    else
    {
        /* Open the log file for appending. */
        fp = fopen( nwipe_options.logfile, "a" );

        if( fp == NULL )
        {
            fprintf( stderr, "nwipe_log: Unable to open '%s' for logging.\n", nwipe_options.logfile );
            r = pthread_mutex_unlock( &mutex1 );
            if( r != 0 )
            {
                fprintf( stderr, "nwipe_log: pthread_mutex_unlock failed. Code %i \n", r );
                return;
            }
        }

        /* Get the file descriptor of the log file. */
        fd = fileno( fp );

        /* Block and lock. */
        r = flock( fd, LOCK_EX );

        if( r != 0 )
        {
            perror( "nwipe_log: flock:" );
            fprintf( stderr, "nwipe_log: Unable to lock '%s' for logging.\n", nwipe_options.logfile );
            r = pthread_mutex_unlock( &mutex1 );
            if( r != 0 )
            {
                fprintf( stderr, "nwipe_log: pthread_mutex_unlock failed. Code %i \n", r );

                /* Unlock the file. */
                r = flock( fd, LOCK_UN );
                fclose( fp );
                return;
            }
        }

        fprintf( fp, "%s\n", log_lines[log_current_element] );

        /* Unlock the file. */
        r = flock( fd, LOCK_UN );

        if( r != 0 )
        {
            perror( "nwipe_log: flock:" );
            fprintf( stderr, "Error: Unable to unlock '%s' after logging.\n", nwipe_options.logfile );
        }

        /* Close the stream. */
        r = fclose( fp );

        if( r != 0 )
        {
            perror( "nwipe_log: fclose:" );
            fprintf( stderr, "Error: Unable to close '%s' after logging.\n", nwipe_options.logfile );
        }
    }

    log_current_element++;

    r = pthread_mutex_unlock( &mutex1 );
    if( r != 0 )
    {
        fprintf( stderr, "nwipe_log: pthread_mutex_unlock failed. Code %i \n", r );
    }
    return;

} /* nwipe_log */

void nwipe_perror( int nwipe_errno, const char* f, const char* s )
{
    /**
     * Wrapper for perror().
     *
     * We may wish to tweak or squelch this later.
     *
     */

    nwipe_log( NWIPE_LOG_ERROR, "%s: %s: %s", f, s, strerror( nwipe_errno ) );

} /* nwipe_perror */

int nwipe_log_sysinfo()
{
    FILE* fp;
    char path[256];
    int len;
    int r;  // A result buffer.

    /*
     * Remove or add keywords to be searched, depending on what information is to
     * be logged, making sure the last entry in the array is a NULL string. To remove
     * an entry simply comment out the keyword with //
     */
    char dmidecode_keywords[][24] = {
        "bios-version",
        "bios-release-date",
        "system-manufacturer",
        "system-product-name",
        "system-version",
        "system-serial-number",
        "system-uuid",
        "baseboard-manufacturer",
        "baseboard-product-name",
        "baseboard-version",
        "baseboard-serial-number",
        "baseboard-asset-tag",
        "chassis-manufacturer",
        "chassis-type",
        "chassis-version",
        "chassis-serial-number",
        "chassis-asset-tag",
        "processor-family",
        "processor-manufacturer",
        "processor-version",
        "processor-frequency",
        ""  // terminates the keyword array. DO NOT REMOVE
    };

    char dmidecode_command[] = "dmidecode -s %s";
    char dmidecode_command2[] = "/sbin/dmidecode -s %s";
    char dmidecode_command3[] = "/usr/bin/dmidecode -s %s";
    char* p_dmidecode_command;

    char cmd[sizeof( dmidecode_keywords ) + sizeof( dmidecode_command2 )];

    unsigned int keywords_idx;

    keywords_idx = 0;

    p_dmidecode_command = 0;

    if( system( "which dmidecode > /dev/null 2>&1" ) )
    {
        if( system( "which /sbin/dmidecode > /dev/null 2>&1" ) )
        {
            if( system( "which /usr/bin/dmidecode > /dev/null 2>&1" ) )
            {
                nwipe_log( NWIPE_LOG_WARNING, "Command not found. Install dmidecode !" );
            }
            else
            {
                p_dmidecode_command = &dmidecode_command3[0];
            }
        }
        else
        {
            p_dmidecode_command = &dmidecode_command2[0];
        }
    }
    else
    {
        p_dmidecode_command = &dmidecode_command[0];
    }

    if( p_dmidecode_command != 0 )
    {

        /* Run the dmidecode command to retrieve each dmidecode keyword, one at a time */
        while( dmidecode_keywords[keywords_idx][0] != 0 )
        {
            sprintf( cmd, p_dmidecode_command, &dmidecode_keywords[keywords_idx][0] );
            fp = popen( cmd, "r" );
            if( fp == NULL )
            {
                nwipe_log( NWIPE_LOG_WARNING, "nwipe_log_sysinfo: Failed to create stream to %s", cmd );
                return 1;
            }
            /* Read the output a line at a time - output it. */
            while( fgets( path, sizeof( path ) - 1, fp ) != NULL )
            {
                /* Remove any trailing return from the string, as nwipe_log automatically adds a return */
                len = strlen( path );
                if( path[len - 1] == '\n' )
                {
                    path[len - 1] = 0;
                }
                nwipe_log( NWIPE_LOG_NOTICE, "%s = %s", &dmidecode_keywords[keywords_idx][0], path );
            }
            /* close */
            r = pclose( fp );
            if( r > 0 )
            {
                nwipe_log( NWIPE_LOG_WARNING,
                           "nwipe_log_sysinfo(): dmidecode failed, \"%s\" exit status = %u",
                           cmd,
                           WEXITSTATUS( r ) );
                return 1;
            }
            keywords_idx++;
        }
    }
    return 0;
}

void nwipe_log_summary( nwipe_context_t** ptr, int nwipe_selected )
{
    int i;
    int idx_src;
    int idx_dest;
    char device[7];
    char status[9];
    char throughput[13];
    char total_throughput_string[13];
    char summary_top_border[256];
    char summary_top_column_titles[256];
    char blank[3];
    char verify[3];
    // char duration[5];
    char duration[314];
    char model[18];
    char serial_no[20];
    char exclamation_flag[2];
    int hours;
    int minutes;
    int seconds;
    u64 total_duration_seconds;
    u64 total_throughput;
    nwipe_context_t** c;
    c = ptr;

    exclamation_flag[0] = 0;
    device[0] = 0;
    status[0] = 0;
    throughput[0] = 0;
    summary_top_border[0] = 0;
    summary_top_column_titles[0] = 0;
    blank[0] = 0;
    verify[0] = 0;
    duration[0] = 0;
    model[0] = 0;
    serial_no[0] = 0;
    hours = 0;
    minutes = 0;
    seconds = 0;

    /* A time buffer. */
    time_t t;

    /* A pointer to the system time struct. */
    struct tm* p;

    /* Nothing to do, user didn't select any devices */
    if( nwipe_selected == 0 )
    {
        return;
    }

    /* initialise */
    total_throughput = 0;

    /* Get the current time. */
    t = time( NULL );
    p = localtime( &t );
    /* IMPORTANT: Keep maximum columns (line length) to 80 characters for use with 80x30 terminals, Shredos, ALT-F2 etc
     * --------------------------------01234567890123456789012345678901234567890123456789012345678901234567890123456789-*/
    nwipe_log( NWIPE_LOG_NOTIMESTAMP, "" );
    nwipe_log( NWIPE_LOG_NOTIMESTAMP,
               "********************************************************************************" );
    nwipe_log( NWIPE_LOG_NOTIMESTAMP, "! Device | Status | Thru-put | HH:MM:SS | Model/Serial Number" );
    nwipe_log( NWIPE_LOG_NOTIMESTAMP,
               "--------------------------------------------------------------------------------" );
    /* Example layout:
     *                                "!    sdv |--FAIL--|  120MB/s | 01:22:01 | WD6788.8488YNHj/ZX677888388-N       "
     * ); "     sdv | Erased |  120MB/s | 01:25:04 | WD6784.8488JKGG/ZX677888388-N       " ); "     sdv | Erased |
     * 120MB/s | 01:19:07 | WD6788.848HHDDR/ZX677888388-N       " ); End of Example layout */

    for( i = 0; i < nwipe_selected; i++ )
    {
        /* Device name, strip any prefixed /dev/.. leaving up to 6 right justified
         * characters eg "   sda", prefixed with space to 6 characters, note that
         * we are processing the strings right to left */

        idx_dest = 6;
        device[idx_dest--] = 0;
        idx_src = strlen( c[i]->device_name );
        idx_src--;

        while( idx_dest >= 0 )
        {
            /* if the device name contains a / start prefixing spaces */
            if( c[i]->device_name[idx_src] == '/' )
            {
                device[idx_dest--] = ' ';
                continue;
            }
            if( idx_src >= 0 )
            {
                device[idx_dest--] = c[i]->device_name[idx_src--];
            }
        }
        extern int user_abort;

        /* Any errors ? if so set the exclamation_flag and fail message,
         * All status messages should be eight characters EXACTLY !
         */
        if( c[i]->result < 0 )
        {
            strncpy( exclamation_flag, "!", 1 );
            exclamation_flag[1] = 0;

            strncpy( status, "-FAILED-", 8 );
            status[8] = 0;
        }
        else
        {

            if( c[i]->pass_errors != 0 )
            {
                strncpy( exclamation_flag, "!", 1 );
                exclamation_flag[1] = 0;

                strncpy( status, "-FAILED-", 8 );
                status[8] = 0;
            }
            else
            {
                if( user_abort == 1 )
                {
                    strncpy( exclamation_flag, "!", 1 );
                    exclamation_flag[1] = 0;

                    strncpy( status, "UABORTED", 8 );
                    status[8] = 0;
                }
                else
                {
                    strncpy( exclamation_flag, " ", 1 );
                    exclamation_flag[1] = 0;

                    strncpy( status, " Erased ", 8 );
                    status[8] = 0;
                }
            }
        }

        /* Determine the size of throughput so that the correct nomenclature can be used */
        Determine_C_B_nomenclature( c[i]->throughput, throughput, 13 );

        /* Add this devices throughput to the total throughput */
        total_throughput += c[i]->throughput;

        /* Retrieve the duration of the wipe in seconds and convert to hours and minutes and seconds */

        if( c[i]->start_time != 0 && c[i]->end_time != 0 )
        {
            /* For a summary when the wipe has finished */
            c[i]->duration = difftime( c[i]->end_time, c[i]->start_time );
        }
        else
        {
            if( c[i]->start_time != 0 && c[i]->end_time == 0 )
            {
                /* For a summary in the event of a system shutdown */
                c[i]->duration = difftime( t, c[i]->start_time );
            }
        }

        total_duration_seconds = (u64) c[i]->duration;

        /* Convert binary seconds into three binary variables, hours, minutes and seconds */
        convert_seconds_to_hours_minutes_seconds( total_duration_seconds, &hours, &minutes, &seconds );

        /* Device Model */
        strncpy( model, c[i]->device_model, 17 );
        model[17] = 0;

        /* Serial No. */
        strncpy( serial_no, c[i]->device_serial_no, 20 );
        model[17] = 0;

        nwipe_log( NWIPE_LOG_NOTIMESTAMP,
                   "%s %s |%s| %s/s | %02i:%02i:%02i | %s/%s",
                   exclamation_flag,
                   device,
                   status,
                   throughput,
                   hours,
                   minutes,
                   seconds,
                   model,
                   serial_no );
    }

    /* Determine the size of throughput so that the correct nomenclature can be used */
    Determine_C_B_nomenclature( total_throughput, total_throughput_string, 13 );

    /* Blank abreviations used in summary table B=blank, NB=no blank */
    if( nwipe_options.noblank )
    {
        strcpy( blank, "NB" );
    }
    else
    {
        strcpy( blank, "B" );
    }

    /* Verify abreviations used in summary table */
    switch( nwipe_options.verify )
    {
        case NWIPE_VERIFY_NONE:
            strcpy( verify, "NV" );
            break;

        case NWIPE_VERIFY_LAST:
            strcpy( verify, "VL" );
            break;

        case NWIPE_VERIFY_ALL:
            strcpy( verify, "VA" );
            break;
    }

    nwipe_log( NWIPE_LOG_NOTIMESTAMP,
               "--------------------------------------------------------------------------------" );
    nwipe_log( NWIPE_LOG_NOTIMESTAMP,
               "[%i/%02i/%02i %02i:%02i:%02i] Total Throughput %s/s, %s, %iR+%s+%s",
               1900 + p->tm_year,
               1 + p->tm_mon,
               p->tm_mday,
               p->tm_hour,
               p->tm_min,
               p->tm_sec,
               total_throughput_string,
               nwipe_method_label( nwipe_options.method ),
               nwipe_options.rounds,
               blank,
               verify );
    nwipe_log( NWIPE_LOG_NOTIMESTAMP,
               "********************************************************************************" );
    nwipe_log( NWIPE_LOG_NOTIMESTAMP, "" );
}

void Determine_C_B_nomenclature( u64 speed, char* result, int result_array_size )
{

    /* C_B ? Determine Capacity or Bandwidth nomenclature
     *
     * A pointer to a result character string with a minimum of 13 characters in length
     * should be provided.
     *
     * Outputs a string of the form xxxTB/s, xxxGB/s, xxxMB/s, xxxKB/s B/s depending on the value of 'speed'
     */

    /* Initialise the output array */
    int idx = 0;

    while( idx < result_array_size )
    {
        result[idx++] = 0;
    }

    /* Determine the size of throughput so that the correct nomenclature can be used */
    if( speed >= INT64_C( 1000000000000 ) )
    {
        snprintf( result, result_array_size, "%3llu TB", speed / INT64_C( 1000000000000 ) );
    }
    else if( speed >= INT64_C( 1000000000 ) )
    {
        snprintf( result, result_array_size, "%3llu GB", speed / INT64_C( 1000000000 ) );
    }
    else if( speed >= INT64_C( 1000000 ) )
    {
        snprintf( result, result_array_size, "%3llu MB", speed / INT64_C( 1000000 ) );
    }
    else if( speed >= INT64_C( 1000 ) )
    {
        snprintf( result, result_array_size, "%3llu KB", speed / INT64_C( 1000 ) );
    }
    else
    {
        snprintf( result, result_array_size, "%3llu B", speed / INT64_C( 1 ) );
    }
}

void convert_seconds_to_hours_minutes_seconds( u64 total_seconds, int* hours, int* minutes, int* seconds )
{
    /* Convert binary seconds into binary hours, minutes and seconds */

    if( total_seconds % 60 )
    {
        *minutes = total_seconds / 60;

        *seconds = total_seconds - ( *minutes * 60 );
    }
    else
    {
        *minutes = total_seconds / 60;

        *seconds = 0;
    }
    if( *minutes > 59 )
    {
        *hours = *minutes / 60;
        if( *minutes % 60 )
        {
            *minutes = *minutes - ( *hours * 60 );
        }
        else
        {
            *minutes = 0;
        }
    }
}

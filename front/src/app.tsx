
import * as React from 'react'

import './app.scss';
import Grid from './grid';

export default class App extends React.Component {
    render() {

        return <>
            <div className="header">
                <div className="logo">Lorikeet Dashboard</div>
            </div>
            <Grid />
        </>

    }
}
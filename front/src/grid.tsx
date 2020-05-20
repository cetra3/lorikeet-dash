import * as React from 'react'
import { Responsive, WidthProvider, Layout } from 'react-grid-layout';

const ResponsiveGridLayout = WidthProvider(Responsive);


import './grid.scss'
import Chart from './chart';



interface State {
  is_changing: boolean;
  charts: string[]
}

export default class Grid extends React.Component<{}, State> {

  constructor(props: {}) {
    super(props);

    this.state = {
      is_changing: false,
      charts: []
    };
  }
  
  timeout: any | undefined;

  componentDidMount() {
    fetch("/charts")
      .then(res => res.json())
      .then(charts => {
        this.setState({ charts });
      })
  }

  onResizeStart = () => {
    this.setState({ is_changing: true });
  }

  onResizeEnd = () => {

    if(this.timeout != undefined) {
      clearTimeout(this.timeout);
    }

    this.timeout = setTimeout(() => {
      this.setState({ is_changing: false });
    }, 50)
  }

  onWidthChange = () => {

    if(this.state.is_changing == false) {
      this.setState({ is_changing: true });
    }

    if(this.timeout != undefined) {
      clearTimeout(this.timeout);
    }

    this.timeout = setTimeout(() => {
      this.setState({ is_changing: false });
    }, 50)
  }

  render() {
    // layout is an array of objects, see the demo for more complete usage
    const layout: Layout[] = this.state.charts.map((val, i) => { return { i: val, x: (i * 6) % 12, y: 0, w: 6, h: 2 } })
    return (
      <div className="grid-container">
        <ResponsiveGridLayout onWidthChange={this.onWidthChange} onResizeStart={this.onResizeStart} onResizeStop={this.onResizeEnd} className="layout" layouts={{ lg: layout }}>
          {this.state.charts.map(val => <div key={val}><Chart changing={this.state.is_changing} name={val} /></div>)}
        </ResponsiveGridLayout>
      </div>
    )
  }
}